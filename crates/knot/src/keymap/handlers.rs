//! The handlers behind the configurable shortcuts, except Open Command
//! Center, which keeps its handler beside the Window menu's other opener in
//! `app_bootstrap`.
//!
//! Only Select workspace N is global here: it works from any window and with
//! none open. The workspace-scoped ones (Select agent N, Focus agent input,
//! Jump to bottom, the panel toggles) are registered on the workspace
//! window's root element instead, by `WorkspaceWindow::with_shortcut_actions`,
//! and only while each applies. That is what makes the View menu's items
//! disable where their shortcut would do nothing: macOS asks
//! `App::is_action_available`, and a global listener answers yes everywhere
//! (`app-menu`).

use std::sync::Arc;

use gpui_kit::App;
use parking_lot::Mutex;

use crate::keymap::*;
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
