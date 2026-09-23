//! The workspace window entity: what one holds for the life of a window, and
//! what it tears down when the window closes.
//!
//! Every sibling module in `workspace_window` hangs its `impl` off this
//! struct, so the fields are `pub(super)` - visible across the module, not
//! beyond it.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::Entity;
use gpui_kit::ListState;
use gpui_kit::Subscription;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::resizable::ResizableState;
use knot_terminal::PtyTransport;
use knot_terminal::TerminalSession;
use parking_lot::Mutex;
use uuid::Uuid;

use super::panel;
use super::prompt_queue::QueuedPanelPrompt;
use super::view_mode::WorkspaceViewMode;
use crate::dashboard;
use crate::panel_session;
use crate::panel_state;

/// A delivery result on its way back from the runtime: the agent whose
/// queue it belongs to, the queue entry it answers, and how the prompt
/// fared. The entry is named by id rather than by its text so two prompts
/// that read the same do not collect each other's results.
pub(super) type PanelPromptResult = (Uuid, Uuid, Result<(), String>);

pub(crate) struct WorkspaceWindow {
    /// Last known diff stat per agent, refreshed off the render path - see
    /// `refresh_diff_stats`.
    pub(super) diff_stats:                       crate::diff_stats::DiffStatsCache,
    /// Last known state per recorded pull request URL, refreshed off the
    /// render path and only while the Pull Requests view is showing - see
    /// `refresh_pull_request_states`. Never persisted: a merged pull request
    /// shown as open after a restart is worse than a blank.
    pub(super) pull_request_states:              crate::pull_request_state::PullRequestStateCache,
    /// What the last `gh` probe found, and so which single message the Pull
    /// Requests view shows. Probed once per view opening rather than once
    /// per row.
    pub(super) forge_status:                     crate::pull_request_state::ForgeStatus,
    /// Set when a pull request could not be handed to a browser, so the view
    /// can say so. A click that silently did nothing reads as a broken row.
    pub(super) pull_request_open_failed:         bool,
    /// Agents whose PTY process has exited, queued by the reader thread and
    /// drained by the repaint poll - the callback runs off the main thread
    /// and cannot touch the view directly, the same hand-off
    /// `clipboard_writes` uses.
    pub(super) exited_sessions:                  Arc<Mutex<Vec<Uuid>>>,
    /// Keeps the window-bounds observer alive for this window's lifetime.
    pub(super) window_bounds_subscription:       Option<gpui_kit::Subscription>,
    /// Which config selector's popover is open, by element id, or `None`
    /// when none is. One shared flag used to back all three: because every
    /// selector's `on_open_change` wrote it and the permission selector
    /// read it, clicking Model or Effort opened the *permission* menu.
    pub(super) open_config_selector:             Option<&'static str>,
    pub(super) store:                            Arc<Mutex<knot_agents::AgentStore>>,
    /// Agent-to-agent messages, for the unread badge and the idle-time
    /// delivery nudge (`mcp-messaging`). Shared with the MCP server, which
    /// is what writes to it.
    pub(super) messages:                         Arc<Mutex<knot_messaging::MessageStore>>,
    /// The last message each agent has been nudged about, so an unread
    /// inbox produces one prompt rather than one per idle poll.
    pub(super) nudged_messages:                  BTreeMap<Uuid, Uuid>,
    /// The last awaiting-input message each agent was notified about.
    ///
    /// `Effect::AwaitingInput` fires on every status event reporting Input,
    /// not only on the transition into it, so a prompt the user has not
    /// answered keeps arriving. This is what makes the second one a repeat
    /// rather than news, per `desktop-notifications`' suppression rule.
    pub(super) notified_awaiting:                BTreeMap<Uuid, String>,
    pub(super) settings:                         knot_core::Settings,
    pub(super) workspace_id:                     Uuid,
    pub(super) selected_agent:                   Option<Uuid>,
    pub(super) sessions: BTreeMap<Uuid, Arc<Mutex<TerminalSession<PtyTransport>>>>,
    pub(super) panel_states: BTreeMap<Uuid, Arc<Mutex<panel_state::PanelState>>>,
    /// `TerminalSession::spawn_pty` runs `tokio::spawn` for the activity
    /// tracker; the UI thread has no tokio runtime of its own, so enter
    /// this one around each spawn (see `ensure_session`).
    pub(super) runtime:                          tokio::runtime::Runtime,
    /// Backs the divider between the sidebar and the content column.
    ///
    /// The window owns it rather than letting the group keep its own keyed
    /// state inside the element tree: two things outside the group read the
    /// sidebar's width - the compact predicate and the terminal pane's
    /// geometry - and a window-held entity gives both the same source.
    pub(super) sidebar_resize:                   Entity<ResizableState>,
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
    pub(super) root_focus:                       gpui_kit::FocusHandle,
    /// Focus target for the terminal grid pane - key events only reach
    /// `dispatch_key` while this is focused (click the pane to focus it).
    pub(super) terminal_focus:                   gpui_kit::FocusHandle,
    /// OSC 52 clipboard-store requests, queued by `ensure_session`'s
    /// `on_grid_event` (which runs on the PTY reader thread) and drained
    /// by a polling loop onto the OS pasteboard via GPUI's main-thread
    /// clipboard API - the same background-thread-to-main-thread hand-off
    /// pattern `SettingsWindow` already uses for the native font panel.
    pub(super) clipboard_writes:                 Arc<Mutex<Vec<String>>>,
    /// Live ACP connections for Panel-mode agents, keyed by agent id -
    /// independent of `sessions` (the terminal PTYs), per the
    /// `acp-panel-ui` "Switch to Terminal mid-turn" scenario: an entry
    /// here persists across a view-mode toggle, only stopped on restart.
    pub(super) panel_sessions: BTreeMap<Uuid, Arc<Mutex<panel_session::PanelSessionSlot>>>,
    /// The lifecycle phase each panel session was in the last time the
    /// repaint poll looked, so a slot moving between phases repaints - see
    /// `panel_needs_repaint`.
    pub(super) panel_phases:                     BTreeMap<Uuid, panel_session::PanelPhase>,
    /// Which spinner frame the working indicators were last repainted on -
    /// see `spinner_repaint_due`.
    pub(super) last_spinner_frame:               u128,
    /// The agent whose composer this window last *focused*, or `None` when
    /// the last frame showed no composer at all - see `prepare_frame`.
    ///
    /// It records what focus was taken for, not where focus is now. Those
    /// differ the moment the user clicks anything else, and that is the
    /// point: the frame compares this against the composer it is about to
    /// show, so focus is taken once on the transition into an agent and
    /// never pulled back while the user is working elsewhere in the window.
    /// Reading where focus actually is would undo that.
    pub(super) focused_composer:                 Option<Uuid>,
    /// One prompt-entry input per Panel-mode agent that has been viewed,
    /// created lazily. Not part of `Agent`/persistence - purely UI state.
    /// A `Textarea` (not a single-line `Input`) so the expand/collapse
    /// control can grow the same entity's visible height without losing
    /// in-progress text, rather than swapping to a second entity.
    pub(super) panel_prompt_inputs:              BTreeMap<Uuid, Entity<TextareaState>>,
    /// Keeps each prompt input's `PressEnter` subscription alive for the
    /// life of the entity it was created for (dropping a `Subscription`
    /// cancels it).
    pub(super) panel_prompt_input_subscriptions: BTreeMap<Uuid, Subscription>,
    pub(super) panel_prompt_queues:              BTreeMap<Uuid, Vec<QueuedPanelPrompt>>,
    pub(super) panel_stopping:                   BTreeSet<Uuid>,
    pub(super) panel_prompt_results:             Arc<Mutex<Vec<PanelPromptResult>>>,
    /// One virtualized conversation list per Panel-mode agent that has
    /// been viewed, created lazily - the `ListState` backing
    /// `render_panel`'s virtualization, and the target of the response
    /// action bar's scroll-to-user/scroll-to-top controls and the track
    /// toggle's auto-scroll.
    pub(super) panel_lists:                      BTreeMap<Uuid, ListState>,
    /// The item count each `panel_lists` entry was last reconciled to, so
    /// `render_panel_pane` can `splice` only the rows that actually
    /// changed and leave off-screen rows' measured heights alone.
    pub(super) panel_list_row_counts:            BTreeMap<Uuid, usize>,
    /// Files/images attached via the input area's add-context control,
    /// pending the next send - cleared once the prompt is submitted.
    pub(super) panel_pending_context:            BTreeMap<Uuid, Vec<PathBuf>>,
    /// Panel-mode agent ids whose input area is expanded to the larger
    /// multi-line editing size; absence means collapsed (the default).
    pub(super) panel_input_expanded:             BTreeSet<Uuid>,
    /// The slash lookup's state per Panel-mode agent: the memoized
    /// command/skill registry, which entry is selected, and the token Esc
    /// closed it on. Created on the agent's first lookup, since building it
    /// reads skill roots off disk - see `panel::lookup`.
    pub(super) panel_lookups:                    BTreeMap<Uuid, panel::lookup::PanelLookup>,
    /// This window's handle, so the poll can tell whether it is the active
    /// window before replacing the app-wide menu bar - two open workspace
    /// windows must not fight over whose selection the Agents menu shows.
    pub(super) window_handle:                    AnyWindowHandle,
    /// The last name written to this window's OS title, so `render` can skip
    /// a `set_window_title` that would change nothing.
    ///
    /// A cache of an *output*, not a copy of the state: the title bar and the
    /// OS title both come from the store every frame, so if this ever drifts
    /// the cost is a redundant AppKit call, never a wrong name. That is what
    /// separates it from the settings snapshot in issue #238, where the copy
    /// *is* what gets read and written back.
    pub(super) titled_as:                        String,
    /// The processes section's state per agent that has one: whether it is
    /// open, the last sample, the last failure, and any termination in
    /// flight. One struct per agent rather than a map per field, so teardown
    /// has one entry to prune.
    pub(super) process_sections: BTreeMap<Uuid, crate::agent_processes::ProcessSection>,
    /// Where the sampling task publishes, and the main thread drains.
    pub(super) process_publish:                  crate::agent_processes::PublishSlot,
    /// The generation this window has already drained, so a poll tick that
    /// finds the same pass again is not mistaken for news.
    pub(super) process_generation:               u64,
    /// When the last sample was asked for, so the poll - which ticks thirty
    /// times a second - runs `ps` on the sampler's cadence instead.
    pub(super) process_sampled_at:               Option<std::time::Instant>,
    /// Set while a sample is in flight, so a slow `ps` is not asked for
    /// twice. The same discipline `RefreshCache::claim_refresh` applies to
    /// `git diff`.
    pub(super) process_sampling:                 Arc<std::sync::atomic::AtomicBool>,
    /// Terminations that were refused, queued by the blocking task and
    /// drained by the poll - the same off-main-thread hand-off
    /// `clipboard_writes` and `exited_sessions` use. Agent, PID, reason.
    pub(super) process_failures:                 Arc<Mutex<Vec<(Uuid, u32, String)>>>,
    pub(super) view_mode:                        WorkspaceViewMode,
    pub(super) dashboard_sort:                   dashboard::DashboardSort,
    /// The sidebar's one error line, for a failure the user caused and can
    /// act on - currently only a sidebar width that could not be saved.
    pub(super) error:                            Option<String>,
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
        for (id, session) in &self.sessions {
            {
                let mut session = session.lock();
                // As in `remove_session`: the drop below is the teardown,
                // this is only the polite half of it.
                if let Err(error) = session.shutdown() {
                    eprintln!("failed to shut down agent {id}'s terminal: {error}");
                }
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
