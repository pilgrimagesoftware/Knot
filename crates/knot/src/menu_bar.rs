//! What the menu bar was last built from, and which window it belongs to.
//!
//! Contract: `openspec/specs/app-menu/spec.md`.
//!
//! A [`gpui_kit::Menu`] is a static snapshot: submenu contents, checkmarks
//! and a submenu parent's enabled state do not re-evaluate on their own, so
//! the bar is rebuilt whenever the facts behind them change. This is the
//! record those rebuilds compare against.

use gpui_kit::App;

use crate::agent_menu::AgentMenuSnapshot;
use crate::view_menu::ViewMenuSnapshot;
use crate::view_menu::numbered_workspaces;

/// Everything the menu bar's dynamic parts are built from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct MenuBarSnapshot {
    pub(crate) agents: AgentMenuSnapshot,
    pub(crate) view:   ViewMenuSnapshot,
}

/// Which window's state the menu bar is currently showing, and what it was
/// built from.
///
/// The menu bar is app-wide but the selection it acts on belongs to one
/// window, so ownership has to be recorded somewhere both windows can see:
/// the active window claims it, and only the window that holds it may give
/// it up. Without that, two open workspace windows would overwrite each
/// other's submenus on alternating polls.
#[derive(Default)]
pub(crate) struct MenuBarState {
    pub(crate) owner:    Option<gpui_kit::AnyWindowHandle>,
    pub(crate) snapshot: MenuBarSnapshot,
}

impl gpui_kit::Global for MenuBarState {}

/// Rebuilds the menu bar if the workspace list it shows is out of date,
/// keeping the owner and everything else it was built from.
///
/// For the workspace manager, where workspaces are created, renamed, deleted
/// and reordered. It has no poll of its own, and while it is focused no
/// workspace window owns the bar to notice the change, so Select Workspace
/// would list the old names until one did (`app-menu`).
///
/// Takes the store rather than the list so the lock is released before the
/// bar is rebuilt: nothing here needs it held across a call into AppKit.
pub(crate) fn refresh_menu_bar_workspaces(store: &parking_lot::Mutex<knot_agents::AgentStore>,
                                          cx: &mut App) {
    let workspaces = numbered_workspaces(&store.lock());
    let Some(state) = cx.try_global::<MenuBarState>()
    else {
        return;
    };
    if state.snapshot.view.workspaces == workspaces {
        return;
    }
    let owner = state.owner;
    let mut snapshot = state.snapshot.clone();
    snapshot.view.workspaces = workspaces;
    cx.set_global(MenuBarState { owner,
                                 snapshot: snapshot.clone() });
    crate::app_bootstrap::set_app_menus(&snapshot, cx);
}
