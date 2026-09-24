//! Building the `WorkspaceWindow` view, separately from opening the window
//! around it.
//!
//! [`super::open`] does three things that only a real workspace window
//! wants: it finds or raises an existing window through the registry,
//! decorates an OS window and places it on a display, and starts a session
//! for every agent in the workspace. None of that is construction, and a
//! test that wants a `WorkspaceWindow` wants none of it - `ensure_session`
//! alone spawns a PTY.
//!
//! So the struct literal lives here, behind [`WorkspaceWindow::new`], and
//! `open` supplies a [`WorkspaceWindowSeed`] like any other caller. One
//! construction path rather than two that drift: a field added to the view
//! is added here and both callers get it.
//!
//! See `crate::tests::workspace_window` for the caller that is not `open`.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

#[cfg(test)]
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::resizable::ResizableState;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::dashboard;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::terminal_font::TerminalFont;

/// What a `WorkspaceWindow` needs from its caller.
///
/// A struct rather than eight arguments, per the workspace's argument-count
/// convention. Everything else the view starts with is a default, and lives
/// in [`WorkspaceWindow::new`] rather than here, so a caller cannot get a
/// window into a state `open` would never produce.
pub(crate) struct WorkspaceWindowSeed {
    pub(crate) store:            Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) messages:         Arc<Mutex<knot_messaging::MessageStore>>,
    pub(crate) workspace_id:     Uuid,
    pub(crate) selected_agent:   Option<Uuid>,
    /// The name the OS window was titled with, kept so the view can tell
    /// whether the title it would draw has changed.
    pub(crate) titled_as:        String,
    pub(crate) sidebar_resize:   Entity<ResizableState>,
    pub(crate) clipboard_writes: Arc<Mutex<Vec<String>>>,
    pub(crate) exited_sessions:  Arc<Mutex<Vec<Uuid>>>,
}

impl WorkspaceWindow {
    /// The view, with nothing running in it.
    ///
    /// No session is started, nothing is registered, and no poll is
    /// spawned - [`super::open`] does all three after this returns, in that
    /// order. A caller that skips them gets a window that draws and holds
    /// state but has no agents running, which is what a test wants.
    pub(crate) fn new(seed: WorkspaceWindowSeed, window: &mut Window, cx: &mut Context<Self>)
                      -> Self {
        Self { exited_sessions:                  seed.exited_sessions,
               window_bounds_subscription:       None,
               diff_stats:                       crate::diff_stats::DiffStatsCache::default(),
               git_panel_open:                   BTreeSet::new(),
               git_status:                       crate::git_panel::state::GitStatusCache::default(),
               git_diffs:                        crate::git_panel::state::GitDiffCache::default(),
               git_selection:                    BTreeMap::new(),
               git_watches:                      BTreeMap::new(),
               git_watch_dirty:                  BTreeMap::new(),
               git_panel_width:                  BTreeMap::new(),
               git_action_error:                 BTreeMap::new(),
               pending_git_actions:              BTreeMap::new(),
               pending_git_commits:              BTreeMap::new(),
               git_diff_lists:                   BTreeMap::new(),
               git_diff_row_counts:              BTreeMap::new(),
               git_panel_resize:                 BTreeMap::new(),
               pull_request_states:
                   crate::pull_request_state::PullRequestStateCache::default(),
               forge_status:                     crate::pull_request_state::ForgeStatus::default(),
               pull_request_open_failed:         false,
               open_config_selector:             None,
               store:                            seed.store,
               messages:                         seed.messages,
               nudged_messages:                  BTreeMap::new(),
               notified_awaiting:                BTreeMap::new(),
               workspace_id:                     seed.workspace_id,
               selected_agent:                   seed.selected_agent,
               sessions:                         BTreeMap::new(),
               panel_states:                     BTreeMap::new(),
               runtime:
                   tokio::runtime::Runtime::new().expect("failed to start terminal session runtime"),
               sidebar_resize:                   seed.sidebar_resize,
               root_focus:                       cx.focus_handle(),
               terminal_focus:                   cx.focus_handle(),
               terminal_font:                    TerminalFont::default(),
               clipboard_writes:                 seed.clipboard_writes,
               panel_sessions:                   BTreeMap::new(),
               last_spinner_frame:               0,
               focused_pane:                     None,
               panel_phases:                     BTreeMap::new(),
               panel_prompt_inputs:              BTreeMap::new(),
               panel_selectors:                  BTreeMap::new(),
               panel_selector_subscriptions:     BTreeMap::new(),
               panel_selector_items:             BTreeMap::new(),
               panel_prompt_input_subscriptions: BTreeMap::new(),
               panel_prompt_queues:              BTreeMap::new(),
               panel_stopping:                   BTreeSet::new(),
               panel_prompt_results:             Arc::new(Mutex::new(Vec::new())),
               panel_lists:                      BTreeMap::new(),
               panel_list_row_counts:            BTreeMap::new(),
               window_handle:                    window.window_handle(),
               titled_as:                        seed.titled_as,
               panel_pending_context:            BTreeMap::new(),
               panel_composer_styling:           BTreeMap::new(),
               panel_mentions:                   BTreeMap::new(),
               panel_pending_attachments:        BTreeMap::new(),
               panel_shell_runs:                 Arc::new(Mutex::new(BTreeMap::new())),
               panel_input_expanded:             BTreeSet::new(),
               panel_lookups:                    BTreeMap::new(),
               process_sections:                 BTreeMap::new(),
               process_publish:
                   Arc::new(Mutex::new(crate::agent_processes::Published::default())),
               process_generation:               0,
               process_sampled_at:               None,
               process_sampling:
                   Arc::new(std::sync::atomic::AtomicBool::new(false)),
               process_failures:                 Arc::new(Mutex::new(Vec::new())),
               mcp_sections:                     BTreeMap::new(),
               mcp_in_flight:                    Arc::new(Mutex::new(BTreeSet::new())),
               mcp_results:                      Arc::new(Mutex::new(Vec::new())),
               mcp_handover_terminals:           BTreeMap::new(),
               view_mode:                        WorkspaceViewMode::Terminal,
               dashboard_sort:                   dashboard::DashboardSort::default(),
               error:                            None, }
    }
}

/// A `WorkspaceWindow` in a test, with nothing running in it.
///
/// Opens a plain GPUI window and builds the view in it through
/// [`WorkspaceWindow::new`] - the same constructor [`super::open`] uses, so a
/// field added to the view reaches this too. What it deliberately does not do
/// is the rest of `open`: no registry entry, no repaint poll, no bounds
/// observer, and no `ensure_session`, which would spawn a PTY and an adapter
/// subprocess per agent.
///
/// The workspace holds no agents. A test that wants one adds it to the store
/// it gets back and puts the id where the case under test reads it, rather
/// than going through the activation path `open` runs.
///
/// `settings` has to be rooted at a temporary directory -
/// `Settings::with_store_root` - for the reason `workspace_window_open`
/// spells out: a `Settings::default()` carries no store paths, so anything
/// that reaches a persist writes over the developer's own workspaces and
/// agents.
#[cfg(test)]
pub(in crate::workspace_window) fn open_test_window(
    cx: &mut gpui_kit::TestAppContext, settings: knot_core::Settings)
    -> (gpui_kit::VisualTestContext, Entity<WorkspaceWindow>, Arc<Mutex<knot_agents::AgentStore>>) {
    let store = Arc::new(Mutex::new(knot_agents::AgentStore::new()));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let workspace = knot_core::Workspace { id:        Uuid::new_v4(),
                                           name:      "Seam".to_string(),
                                           color_hex: "#123456".to_string(),
                                           agent_ids: Vec::new(), };
    let workspace_id = workspace.id;
    store.lock().add_workspace(workspace);

    let mut view = None;
    let window = {
        let view = &mut view;
        let store = Arc::clone(&store);
        let messages = Arc::clone(&messages);
        cx.update(|cx| {
              gpui_kit::init(cx);
              crate::settings_global::install(settings, cx);
              cx.open_window(gpui_kit::WindowOptions::default(), |window, cx| {
                    let sidebar_resize = cx.new(|_| ResizableState::default());
                    let built = cx.new(|cx| {
                                      WorkspaceWindow::new(WorkspaceWindowSeed {
                            store,
                            messages,
                            workspace_id,
                            selected_agent: None,
                            titled_as: "Seam".to_string(),
                            sidebar_resize,
                            clipboard_writes: Arc::new(Mutex::new(Vec::new())),
                            exited_sessions: Arc::new(Mutex::new(Vec::new())),
                        },
                                                           window,
                                                           cx)
                                  });
                    *view = Some(built.clone());
                    cx.new(|cx| gpui_kit::component::Root::new(built, window, cx))
                })
                .expect("the workspace window should open")
          })
    };

    let cx = gpui_kit::VisualTestContext::from_window(window.into(), cx);
    (cx, view.expect("the view was built"), store)
}

#[cfg(test)]
mod tests;
