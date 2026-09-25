//! The handlers behind the configurable shortcuts, except Open Command
//! Center, which keeps its handler beside the Window menu's other opener in
//! `app_bootstrap`.
//!
//! All of them are global. The workspace-scoped ones (Select agent N, Focus
//! agent input, Jump to bottom, the panel toggles) find their window from
//! `cx.active_window()` rather than being registered on the window's root
//! element: once a Dashboard or Pull Requests panel takes over, the composer
//! or terminal that held focus is no longer drawn, focus falls outside the
//! root element, and a handler on that element is off the dispatch path -
//! the shortcut would work until the first time it was needed to leave a
//! panel. In any window that is not a workspace window they find nothing and
//! do nothing (`keybindings`).

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::Context;
use parking_lot::Mutex;

use crate::keymap::*;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;

pub(crate) fn register_global_handlers(store: Arc<Mutex<knot_agents::AgentStore>>,
                                       messages: Arc<Mutex<knot_messaging::MessageStore>>,
                                       cx: &mut App) {
    macro_rules! select_workspace {
        ($($action:ty => $index:expr),* $(,)?) => {$({
            let store = Arc::clone(&store);
            let messages = Arc::clone(&messages);
            cx.on_action(move |_: &$action, cx| select_workspace($index, &store, &messages, cx));
        })*};
    }
    select_workspace!(SelectWorkspace1 => 0,
                      SelectWorkspace2 => 1,
                      SelectWorkspace3 => 2,
                      SelectWorkspace4 => 3,
                      SelectWorkspace5 => 4,
                      SelectWorkspace6 => 5,
                      SelectWorkspace7 => 6,
                      SelectWorkspace8 => 7,
                      SelectWorkspace9 => 8);

    macro_rules! select_agent {
        ($($action:ty => $index:expr),* $(,)?) => {$(
            cx.on_action(|_: &$action, cx| {
                in_active_workspace(cx, |view, cx| view.select_agent_at($index, cx));
            });
        )*};
    }
    select_agent!(SelectAgent1 => 0,
                  SelectAgent2 => 1,
                  SelectAgent3 => 2,
                  SelectAgent4 => 3,
                  SelectAgent5 => 4,
                  SelectAgent6 => 5,
                  SelectAgent7 => 6,
                  SelectAgent8 => 7,
                  SelectAgent9 => 8);

    cx.on_action(|_: &FocusAgentInput, cx| {
          in_active_workspace(cx, |view, cx| view.focus_agent_input(cx));
      });
    cx.on_action(|_: &JumpToBottom, cx| {
          in_active_workspace(cx, |view, cx| view.jump_to_bottom(cx));
      });
    cx.on_action(|_: &ToggleDashboard, cx| {
          in_active_workspace(cx, |view, cx| {
              view.toggle_view(WorkspaceViewMode::Dashboard, cx)
          });
      });
    cx.on_action(|_: &TogglePullRequests, cx| {
          in_active_workspace(cx, |view, cx| {
              view.toggle_view(WorkspaceViewMode::PullRequests, cx);
          });
      });
}

/// Runs `f` on the active window's workspace view; nothing when the active
/// window is not a workspace window, or there is none.
fn in_active_workspace(cx: &mut App,
                       f: impl FnOnce(&mut WorkspaceWindow, &mut Context<WorkspaceWindow>)) {
    let Some(view) = cx.active_window()
                       .and_then(|handle| WindowRegistry::workspace_view_in(handle, cx))
    else {
        return;
    };
    view.update(cx, f);
}

/// Opens or raises the workspace at `index` in the manager's order; nothing
/// when there is no such workspace. `WorkspaceWindow::open` is what keeps it
/// to one window per workspace.
///
/// Deferred, because the shortcut is dispatched from inside the focused
/// window's own update: raising a window is an update of its handle, which
/// fails for the window already being updated, and the registry reads that
/// failure as "closed" and opens a second window for the workspace that was
/// already in front.
fn select_workspace(index: usize, store: &Arc<Mutex<knot_agents::AgentStore>>,
                    messages: &Arc<Mutex<knot_messaging::MessageStore>>, cx: &mut App) {
    let Some(workspace_id) = store.lock()
                                  .workspaces()
                                  .get(index)
                                  .map(|workspace| workspace.id)
    else {
        return;
    };
    let store = Arc::clone(store);
    let messages = Arc::clone(messages);
    cx.defer(move |cx| WorkspaceWindow::open(store, messages, workspace_id, cx));
}
