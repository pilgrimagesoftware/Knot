//! Opening a workspace window: the GPUI window itself, the initial
//! selection, the subscriptions that keep it in step with the store, and
//! the repaint poll that drives everything that changes off the main
//! thread.
//!
//! The poll is the reason this is long. GPUI redraws on notification, and
//! a session's output, a spinner frame and a panel phase change all arrive
//! from other threads, so one timer asks [`super::repaint`]'s predicates
//! what has moved and notifies when something has.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::component::Root;
use gpui_kit::component::resizable::ResizableState;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_state::agent_selection_for_workspace;
use crate::app_support::observe_system_appearance;
use crate::dashboard;
use crate::window_options::reconciled_workspace_bounds;
use crate::window_options::workspace_window_options;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::repaint::spawn_repaint_poll;
use crate::workspace_window::workspace_title;

impl WorkspaceWindow {
    pub(crate) fn open(store: Arc<Mutex<knot_agents::AgentStore>>,
                       messages: Arc<Mutex<knot_messaging::MessageStore>>,
                       settings: knot_core::Settings, workspace_id: Uuid, cx: &mut App) {
        Self::open_with_selection(store, messages, settings, workspace_id, None, cx);
    }

    /// Like `open`, but overrides the agent that would otherwise be picked
    /// by `agent_selection_for_workspace` - used when a caller (e.g. a
    /// Command Center card) already knows which agent the user wants to
    /// land on.
    pub(crate) fn open_with_selection(store: Arc<Mutex<knot_agents::AgentStore>>,
                                      messages: Arc<Mutex<knot_messaging::MessageStore>>,
                                      settings: knot_core::Settings, workspace_id: Uuid,
                                      select_agent: Option<Uuid>, cx: &mut App) {
        // One window per workspace: a second request raises the first rather
        // than opening another, and shows the agent it named if it named one
        // (`openspec/specs/window-lifecycle`). Every route that opens a
        // workspace - the manager, a Command Center card or heading, the agent
        // editor's post-create jump - arrives here, so the check belongs here
        // rather than at each of them.
        let key = WindowKey::Workspace(workspace_id);
        if WindowRegistry::activate(key, cx) {
            if let Some(requested) = select_agent
               && let Some(view) = WindowRegistry::workspace_view(key, cx)
            {
                view.update(cx, |view, cx| view.reveal_agent(requested, cx));
            }
            return;
        }
        // Through the same resolver the title bar renders from, so the OS
        // title and the drawn one agree by construction rather than by two
        // lookups that happen to match. The fallback is only reachable for a
        // workspace that is already gone, whose window draws
        // `workspace.missing` instead.
        let workspace_name =
            workspace_title(&store.lock(), workspace_id).unwrap_or_else(|| "Workspace".to_string());
        let saved_bounds = store.lock().workspace_ui(workspace_id).window_bounds;
        let placed = reconciled_workspace_bounds(saved_bounds, cx);
        let options = workspace_window_options(placed, cx);
        if let Err(error) =
            cx.open_window(options, move |window, cx| {
                  // Every window tracks the OS appearance, so a light/dark flip
                  // re-resolves the system palette and repaints.
                  observe_system_appearance(window);
                  // The OS window title (Mission Control, Cmd+`, Window menu)
                  // is separate from the TitleBar row we draw
                  // ourselves - without this it falls back to
                  // the app's bundle name for every workspace
                  // window.
                  window.set_window_title(&workspace_name);
                  // Order the new window front rather than letting it open
                  // behind whatever has focus - matching what the settings
                  // window already does when it reuses an open one.
                  window.activate_window();
                  let selected_agent = select_agent.or_else(|| {
                                                       let store = store.lock();
                                                       agent_selection_for_workspace(&store,
                                                                                     workspace_id)
                                                   });
                  let sidebar_resize = cx.new(|_| ResizableState::default());
                  let clipboard_writes = Arc::new(Mutex::new(Vec::new()));
                  let exited_sessions: Arc<Mutex<Vec<Uuid>>> = Arc::new(Mutex::new(Vec::new()));
                  let view =
                      cx.new(|cx| {
                            let mut window = WorkspaceWindow {
                    exited_sessions: Arc::clone(&exited_sessions),
                    window_bounds_subscription: None,
                    diff_stats: crate::diff_stats::DiffStatsCache::default(),
                    git_panel_open: BTreeSet::new(),
                    git_status: crate::git_panel::state::GitStatusCache::default(),
                    git_diffs: crate::git_panel::state::GitDiffCache::default(),
                    git_selection: BTreeMap::new(),
                    git_watches: BTreeMap::new(),
                    git_watch_dirty: BTreeMap::new(),
                    git_panel_width: BTreeMap::new(),
                    git_action_error: BTreeMap::new(),
                    pending_git_actions: BTreeMap::new(),
                    pending_git_commits: BTreeMap::new(),
                    git_diff_lists: BTreeMap::new(),
                    git_diff_row_counts: BTreeMap::new(),
                    git_panel_resize: BTreeMap::new(),
                    pull_request_states:
                        crate::pull_request_state::PullRequestStateCache::default(),
                    forge_status: crate::pull_request_state::ForgeStatus::default(),
                    pull_request_open_failed: false,
                    open_config_selector: None,
                    store,
                    messages,
                    nudged_messages: BTreeMap::new(),
                    notified_awaiting: BTreeMap::new(),
                    settings,
                    workspace_id,
                    selected_agent,
                    sessions: BTreeMap::new(),
                    panel_states: BTreeMap::new(),
                    runtime: tokio::runtime::Runtime::new()
                        .expect("failed to start terminal session runtime"),
                    sidebar_resize,
                    root_focus: cx.focus_handle(),
                    terminal_focus: cx.focus_handle(),
                    clipboard_writes: Arc::clone(&clipboard_writes),
                    panel_sessions: BTreeMap::new(),
                    last_spinner_frame: 0,
                    focused_composer: None,
                    panel_phases: BTreeMap::new(),
                    panel_prompt_inputs: BTreeMap::new(),
                    panel_prompt_input_subscriptions: BTreeMap::new(),
                    panel_prompt_queues: BTreeMap::new(),
                    panel_stopping: BTreeSet::new(),
                    panel_prompt_results: Arc::new(Mutex::new(Vec::new())),
                    panel_lists: BTreeMap::new(),
                    panel_list_row_counts: BTreeMap::new(),
                    window_handle: window.window_handle(),
                    titled_as: workspace_name.clone(),
                    panel_pending_context: BTreeMap::new(),
                    panel_composer_styling: BTreeMap::new(),
                    panel_mentions: BTreeMap::new(),
                    panel_input_expanded: BTreeSet::new(),
                    panel_lookups: BTreeMap::new(),
                    process_sections: BTreeMap::new(),
                    process_publish: Arc::new(Mutex::new(
                        crate::agent_processes::Published::default(),
                    )),
                    process_generation: 0,
                    process_sampled_at: None,
                    process_sampling: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    process_failures: Arc::new(Mutex::new(Vec::new())),
                    view_mode: WorkspaceViewMode::Terminal,
                    dashboard_sort: dashboard::DashboardSort::default(),
                    error: None,
                };
                            // Matches the Swift reference: every agent in the
                            // workspace starts its session when the workspace
                            // window opens, not
                            // just the one initially selected.
                            let agent_ids: Vec<Uuid> =
                                {
                                    let store = window.store.lock();
                                    store.workspaces()
                                         .iter()
                                         .find(|workspace| workspace.id == workspace_id)
                                         .map(|workspace| workspace.agent_ids.clone())
                                }.unwrap_or_default();
                            {
                                let mut store = window.store.lock();
                                // The workspace's `active` agents start
                                // here, and only they: `activated` is
                                // runtime-only and loads false, so the
                                // durable mode is consulted on every open.
                                store.activate_on_workspace_open(&agent_ids);
                                // An explicitly requested agent (a Command
                                // Center card) is a selection, and starts
                                // whatever its mode. The workspace's own
                                // restored selection is not, so a `passive`
                                // agent stays stopped across a relaunch.
                                if let Some(requested) = select_agent {
                                    store.set_activated(requested, true);
                                }
                            }
                            for id in agent_ids {
                                window.ensure_session(id);
                                window.ensure_panel_session(id);
                            }
                            window
                        });
                  // Registered from inside the open closure because the
                  // view is only in scope here: `cx.open_window` hands back a
                  // handle to the `Root` wrapper, not to this.
                  WindowRegistry::register(key, window.window_handle(), Some(view.downgrade()), cx);
                  spawn_repaint_poll(view.clone(),
                                     clipboard_writes,
                                     Arc::clone(&exited_sessions),
                                     cx);
                  // Remember where the user puts this workspace's window.
                  // The observer fires continuously through a drag, so the
                  // store's setter reports whether the frame actually
                  // changed and only then is anything written to disk.
                  view.update(cx, |view, cx| {
                          let subscription =
                              cx.observe_window_bounds(window, move |view, window, _cx| {
                                    let bounds = window.window_bounds().get_bounds();
                                    // Our own placement is not a move the user
                                    // made. Writing it back would replace the
                                    // remembered frame with the one we fell
                                    // back to, so a window arranged on a
                                    // display that is merely unplugged would
                                    // lose its place the first time it was
                                    // reopened without it.
                                    if Some(bounds) == placed {
                                        return;
                                    }
                                    let saved =
                                        knot_core::SavedWindowBounds { x:      bounds.origin
                                                                                     .x
                                                                                     .into(),
                                                                       y:      bounds.origin
                                                                                     .y
                                                                                     .into(),
                                                                       width:  bounds.size
                                                                                     .width
                                                                                     .into(),
                                                                       height: bounds.size
                                                                                     .height
                                                                                     .into(), };
                                    let changed =
                                        view.store
                                            .lock()
                                            .set_workspace_window_bounds(workspace_id, saved);
                                    if changed {
                                        // The UI-state document alone: a
                                        // drag must not rewrite the roster.
                                        view.persist_workspace_ui();
                                    }
                                });
                          view.window_bounds_subscription = Some(subscription);
                      });
                  cx.new(|cx| Root::new(view, window, cx))
              })
        {
            eprintln!("failed to open workspace window: {error}");
        }
    }
}
