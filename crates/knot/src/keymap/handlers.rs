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
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;

pub(crate) fn register_global_handlers(store: Arc<Mutex<knot_agents::AgentStore>>, cx: &mut App) {
    macro_rules! select_workspace {
        ($($action:ty => $index:expr),* $(,)?) => {$({
            let store = Arc::clone(&store);
            cx.on_action(move |_: &$action, cx| select_workspace($index, &store, cx));
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

/// Brings the window of the workspace at `index` in the manager's order to
/// the front, if it is open. Nothing when it is not, or when there is no
/// such workspace (#543): opening a workspace is the workspace manager's
/// job, and a numbered shortcut that opened windows put a new one up for
/// every digit pressed by mistake.
///
/// Deferred, because the shortcut is dispatched from inside the focused
/// window's own update: raising a window is an update of its handle, which
/// fails for the window already being updated, and the registry reads that
/// failure as "closed" and forgets the window that was already in front.
fn select_workspace(index: usize, store: &Arc<Mutex<knot_agents::AgentStore>>, cx: &mut App) {
    let Some(workspace_id) = store.lock()
                                  .workspaces()
                                  .get(index)
                                  .map(|workspace| workspace.id)
    else {
        return;
    };
    cx.defer(move |cx| {
          WindowRegistry::activate(WindowKey::Workspace(workspace_id), cx);
      });
}
