use super::*;
use crate::app_bootstrap::{
    PanelOpenPermissionSelector, PanelPermissionAllow, PanelPermissionDeny,
};

mod agents;
mod chrome;
mod creation;
mod menus;
mod open;
mod panel;
mod render;
mod repaint;
mod sessions;
mod terminal_input;

// Re-exported so the rest of the crate keeps reaching these by
// `workspace_window::<name>`, as it did when they lived here.
pub(crate) use chrome::*;
pub(crate) use creation::SelectedAgentHeader;
pub(crate) use menus::*;
/// Which peer view a `WorkspaceWindow` currently shows - the dashboard is a
/// toggleable view of the same window's content, not a dialog or a
/// separate window (see `openspec/changes/dashboard-view/design.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkspaceViewMode {
    #[default]
    Terminal,
    Dashboard,
}

/// Terminal pane geometry - shared by resize and mouse-position translation
/// so they agree on the same grid.
pub(crate) const TERMINAL_SIDEBAR_WIDTH: f32 = 250.;
pub(crate) const TERMINAL_HEADER_HEIGHT: f32 = 64.;

/// Which text size a detail line renders at, and therefore what size its
/// icon has to be.
///
/// The row's detail lines are not all one size - the agent type and persona
/// are `text_xs`, the status and folder are the UI font size - so an icon
/// fixed at one size reads as undersized beside half of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetailLineSize {
    /// `text_xs`, for the type and persona lines.
    Small,
    /// The UI font size, for the status and folder lines.
    Body,
}

pub(crate) mod prompt_queue;

use prompt_queue::{QueuedPanelPrompt, queued_status_label};

/// A delivery result on its way back from the runtime: the agent whose
/// queue it belongs to, the queue entry it answers, and how the prompt
/// fared. The entry is named by id rather than by its text so two prompts
/// that read the same do not collect each other's results.
type PanelPromptResult = (Uuid, Uuid, Result<(), String>);

/// The permission-mode selector's element id - the one selector with
/// risk-tinted labels and a keyboard action of its own, so it needs naming
/// rather than matching on a literal in three places.
const PERMISSION_SELECTOR_ID: &str = "panel-permission-mode-selector";

/// How far the prompt box grows with its content before it starts
/// scrolling, collapsed and expanded. It auto-grows rather than sitting at
/// a fixed height: a fixed height fights the textarea's own layout, so a
/// second line made it scroll and jump on every keystroke instead of
/// simply getting taller.
const PANEL_INPUT_ROWS_COLLAPSED: usize = 6;
const PANEL_INPUT_ROWS_EXPANDED: usize = 20;

/// One sidebar agent row's render inputs, snapshotted out of the store
/// while its lock is held so the row closures don't need it. A struct
/// rather than the tuple this used to be, per the repo convention against
/// wide positional parameter lists.
struct AgentRow {
    id:           Uuid,
    avatar:       String,
    name:         String,
    folder:       String,
    state:        knot_agents::AgentState,
    is_shell:     bool,
    is_companion: bool,
    header_title: String,
    persona_name: Option<String>,
    /// The agent's coding-agent type (`claude`, `opencode`, ...), shown
    /// on the row so a one-letter avatar isn't the only clue to which
    /// agent is running there.
    agent_type:   String,
    /// Whether the agent has a session. Keyed on liveness, not on
    /// activation mode: a `passive` agent that never started and a
    /// deactivated `active` one are in the same position - nothing is
    /// there - and the user needs to know which agents are live, not why
    /// each one is not.
    is_running:   bool,
}

pub(crate) struct WorkspaceWindow {
    /// Last known diff stat per agent, refreshed off the render path - see
    /// `refresh_diff_stats`.
    diff_stats:                       Arc<Mutex<BTreeMap<Uuid, Option<knot_git::DiffStats>>>>,
    /// When each agent's diff stat was last *requested*, so the refresh
    /// runs on a cadence rather than once per render. Main-thread only.
    diff_stats_requested:             BTreeMap<Uuid, std::time::Instant>,
    /// Agents whose PTY process has exited, queued by the reader thread and
    /// drained by the repaint poll - the callback runs off the main thread
    /// and cannot touch the view directly, the same hand-off
    /// `clipboard_writes` uses.
    exited_sessions:                  Arc<Mutex<Vec<Uuid>>>,
    /// Keeps the window-bounds observer alive for this window's lifetime.
    window_bounds_subscription:       Option<gpui_kit::Subscription>,
    /// Set by a finished refresh so the repaint poll redraws the header.
    diff_stats_dirty:                 Arc<std::sync::atomic::AtomicBool>,
    /// Which config selector's popover is open, by element id, or `None`
    /// when none is. One shared flag used to back all three: because every
    /// selector's `on_open_change` wrote it and the permission selector
    /// read it, clicking Model or Effort opened the *permission* menu.
    open_config_selector:             Option<&'static str>,
    store:                            Arc<Mutex<knot_agents::AgentStore>>,
    /// Agent-to-agent messages, for the unread badge and the idle-time
    /// delivery nudge (`mcp-messaging`). Shared with the MCP server, which
    /// is what writes to it.
    messages:                         Arc<Mutex<knot_messaging::MessageStore>>,
    /// The last message each agent has been nudged about, so an unread
    /// inbox produces one prompt rather than one per idle poll.
    nudged_messages:                  BTreeMap<Uuid, Uuid>,
    settings:                         knot_core::Settings,
    workspace_id:                     Uuid,
    selected_agent:                   Option<Uuid>,
    sessions:                         BTreeMap<Uuid, Arc<Mutex<TerminalSession<PtyTransport>>>>,
    panel_states:                     BTreeMap<Uuid, Arc<Mutex<panel_state::PanelState>>>,
    /// `TerminalSession::spawn_pty` runs `tokio::spawn` for the activity
    /// tracker; the UI thread has no tokio runtime of its own, so enter
    /// this one around each spawn (see `ensure_session`).
    runtime:                          tokio::runtime::Runtime,
    /// Focus target for the window's root element.
    ///
    /// Nothing else in this window claims focus until the user clicks a
    /// pane, and a window with focus nowhere is why the Agents menu drew
    /// disabled with an agent selected: macOS validates each item against
    /// the dispatch path to the focused node, and gpui resolves "no focus"
    /// to the dispatch-tree *root*, which sits above the element carrying
    /// those handlers. Focusing the root element puts them back on the
    /// path, and leaves them there once a pane takes focus, since the root
    /// is that pane's ancestor.
    root_focus:                       gpui_kit::FocusHandle,
    /// Focus target for the terminal grid pane - key events only reach
    /// `dispatch_key` while this is focused (click the pane to focus it).
    terminal_focus:                   gpui_kit::FocusHandle,
    /// OSC 52 clipboard-store requests, queued by `ensure_session`'s
    /// `on_grid_event` (which runs on the PTY reader thread) and drained
    /// by a polling loop onto the OS pasteboard via GPUI's main-thread
    /// clipboard API - the same background-thread-to-main-thread hand-off
    /// pattern `SettingsWindow` already uses for the native font panel.
    clipboard_writes:                 Arc<Mutex<Vec<String>>>,
    /// Live ACP connections for Panel-mode agents, keyed by agent id -
    /// independent of `sessions` (the terminal PTYs), per the
    /// `acp-panel-ui` "Switch to Terminal mid-turn" scenario: an entry
    /// here persists across a view-mode toggle, only stopped on restart.
    panel_sessions:                   BTreeMap<Uuid, Arc<Mutex<panel_session::PanelSessionSlot>>>,
    /// The lifecycle phase each panel session was in the last time the
    /// repaint poll looked, so a slot moving between phases repaints - see
    /// `panel_needs_repaint`.
    panel_phases:                     BTreeMap<Uuid, panel_session::PanelPhase>,
    /// Which spinner frame the working indicators were last repainted on -
    /// see `spinner_repaint_due`.
    last_spinner_frame:               u128,
    /// One prompt-entry input per Panel-mode agent that has been viewed,
    /// created lazily. Not part of `Agent`/persistence - purely UI state.
    /// A `Textarea` (not a single-line `Input`) so the expand/collapse
    /// control can grow the same entity's visible height without losing
    /// in-progress text, rather than swapping to a second entity.
    panel_prompt_inputs:              BTreeMap<Uuid, Entity<TextareaState>>,
    /// Keeps each prompt input's `PressEnter` subscription alive for the
    /// life of the entity it was created for (dropping a `Subscription`
    /// cancels it).
    panel_prompt_input_subscriptions: BTreeMap<Uuid, Subscription>,
    panel_prompt_queues:              BTreeMap<Uuid, Vec<QueuedPanelPrompt>>,
    panel_stopping:                   BTreeSet<Uuid>,
    panel_prompt_results:             Arc<Mutex<Vec<PanelPromptResult>>>,
    /// One virtualized conversation list per Panel-mode agent that has
    /// been viewed, created lazily - the `ListState` backing
    /// `render_panel`'s virtualization, and the target of the response
    /// action bar's scroll-to-user/scroll-to-top controls and the track
    /// toggle's auto-scroll.
    panel_lists:                      BTreeMap<Uuid, ListState>,
    /// The item count each `panel_lists` entry was last reconciled to, so
    /// `render_panel_pane` can `splice` only the rows that actually
    /// changed and leave off-screen rows' measured heights alone.
    panel_list_row_counts:            BTreeMap<Uuid, usize>,
    working_indicator_last_repaint:   std::time::Instant,
    /// Files/images attached via the input area's add-context control,
    /// pending the next send - cleared once the prompt is submitted.
    panel_pending_context:            BTreeMap<Uuid, Vec<PathBuf>>,
    /// Panel-mode agent ids whose input area is expanded to the larger
    /// multi-line editing size; absence means collapsed (the default).
    panel_input_expanded:             BTreeSet<Uuid>,
    /// This window's handle, so the poll can tell whether it is the active
    /// window before replacing the app-wide menu bar - two open workspace
    /// windows must not fight over whose selection the Agents menu shows.
    window_handle:                    AnyWindowHandle,
    view_mode:                        WorkspaceViewMode,
    dashboard_sort:                   dashboard::DashboardSort,
    new_agent_name_input:             Entity<InputState>,
    new_agent_folder_input:           Entity<InputState>,
    show_new_agent:                   bool,
    error:                            Option<String>,
}

impl Drop for WorkspaceWindow {
    /// Tears down both launch paths, not just the terminal one.
    ///
    /// `sessions` holds the PTY-backed shell agents; `panel_sessions` holds
    /// the ACP connections, each owning an adapter subprocess. Only the
    /// former was shut down here, so closing a workspace window left one
    /// orphaned adapter per panel agent - the same gap `remove_session`
    /// documents for a single agent, applied to the whole window.
    fn drop(&mut self) {
        for session in self.sessions.values() {
            {
                let mut session = session.lock();
                let _ = session.shutdown();
            }
        }
        // Dropping the slot is what guarantees the teardown: it releases
        // the last `AcpClient`, which releases the transport, whose
        // `kill_on_drop` child then dies. The spawned `stop()` is the
        // polite `session/close` on top of that, and best-effort only -
        // this runtime is itself dropped moments later, so a task that has
        // not started may never run. Nothing depends on it having.
        for slot in std::mem::take(&mut self.panel_sessions).into_values() {
            let handle = match std::mem::replace(&mut *slot.lock(),
                                                 panel_session::PanelSessionSlot::connecting().0)
            {
                panel_session::PanelSessionSlot::Ready(handle) => Some(handle),
                _ => None,
            };
            if let Some(handle) = handle {
                let _runtime_guard = self.runtime.enter();
                self.runtime.spawn(async move { handle.stop().await });
            }
        }
    }
}

impl WorkspaceWindow {}

impl WorkspaceWindow {}
