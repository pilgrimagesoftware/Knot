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
use gpui_kit::ClipboardItem;
use gpui_kit::component::Root;
use gpui_kit::component::resizable::ResizableState;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_state::agent_selection_for_workspace;
use crate::app_support::observe_system_appearance;
use crate::consts;
use crate::dashboard;
use crate::window_options::workspace_window_options;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;

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
        let workspace_name = {
                                 let store = store.lock();
                                 store.workspaces()
                                      .iter()
                                      .find(|workspace| workspace.id == workspace_id)
                                      .map(|workspace| workspace.name.clone())
                             }.unwrap_or_else(|| "Workspace".to_string());
        let saved_bounds = {
            let store = store.lock();
            store.workspaces()
                 .iter()
                 .find(|workspace| workspace.id == workspace_id)
                 .and_then(|workspace| workspace.window_bounds)
        };
        let options = workspace_window_options(saved_bounds, cx);
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
                    panel_phases: BTreeMap::new(),
                    panel_prompt_inputs: BTreeMap::new(),
                    panel_prompt_input_subscriptions: BTreeMap::new(),
                    panel_prompt_queues: BTreeMap::new(),
                    panel_stopping: BTreeSet::new(),
                    panel_prompt_results: Arc::new(Mutex::new(Vec::new())),
                    panel_lists: BTreeMap::new(),
                    panel_list_row_counts: BTreeMap::new(),
                    window_handle: window.window_handle(),
                    working_indicator_last_repaint: std::time::Instant::now(),
                    panel_pending_context: BTreeMap::new(),
                    panel_input_expanded: BTreeSet::new(),
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
                  // Drains OSC 52 clipboard-store requests queued from the PTY
                  // reader thread onto the OS pasteboard (see
                  // `clipboard_writes`'s doc comment), and
                  // repaints the terminal grid - the PTY reader
                  // thread has no way to call `cx.notify()` itself, so without
                  // this the grid only visibly updates on an unrelated UI event
                  // (a keystroke, mouse move), making output look stalled after
                  // e.g. pressing Enter.
                  let notify_view = view.clone();
                  let exited_drain = Arc::clone(&exited_sessions);
                  cx.spawn(async move |cx| {
                        loop {
                            cx.background_executor()
                              .timer(consts::REPAINT_POLL_INTERVAL)
                              .await;
                            let texts = std::mem::take(&mut *clipboard_writes.lock());
                            let exited = std::mem::take(&mut *exited_drain.lock());
                            for text in texts {
                                cx.update(|app| {
                                      app.write_to_clipboard(ClipboardItem::new_string(text));
                                  });
                            }
                            cx.update(|app| {
                                  notify_view.update(app, |view, cx| {
                                                 // A shell companion whose
                                                 // process exited has nothing
                                                 // left to show, so close it
                                                 // rather than leaving a dead
                                                 // pane that looks hung.
                                                 for id in &exited {
                                                     view.remove_agent(*id);
                                                     cx.notify();
                                                 }
                                                 // Messages arrive from
                                                 // the MCP server on another
                                                 // thread; this poll is
                                                 // where an agent going idle
                                                 // is noticed.
                                                 view.deliver_inbox_nudges();
                                                 view.raise_awaiting_notifications(cx);
                                                 let grid_dirty =
                                                     view.selected_agent
                                                         .and_then(|id| view.sessions.get(&id))
                                                         .and_then(|session| session.lock().grid())
                                                         .is_some_and(|grid| {
                                                             grid.lock().take_dirty()
                                                         });
                                                 let panel_dirty = view.panel_needs_repaint();
                                                 let spinner_dirty = view.spinner_repaint_due();
                                                 if grid_dirty || panel_dirty || spinner_dirty {
                                                     cx.notify();
                                                 }
                                                 view.refresh_agents_menu(cx);
                                             });
                              });
                        }
                    })
                    .detach();
                  // Remember where the user puts this workspace's window.
                  // The observer fires continuously through a drag, so the
                  // store's setter reports whether the frame actually
                  // changed and only then is anything written to disk.
                  view.update(cx, |view, cx| {
                          let subscription =
                              cx.observe_window_bounds(window, move |view, window, _cx| {
                                    let bounds = window.window_bounds().get_bounds();
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
                                        view.persist_agents();
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
