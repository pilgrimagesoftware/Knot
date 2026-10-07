//! The menu bar's **Window** menu: Knot's two global windows, one item per
//! open workspace window on its Select Workspace shortcut, and the window
//! commands.
//!
//! Contract: `openspec/specs/app-menu/spec.md`, and `keybindings` for what
//! the workspace items do.
//!
//! The workspace items list only workspaces whose window is open (#543).
//! Select Workspace N raises an open window and does nothing otherwise, and
//! an item cannot be greyed out instead: macOS enables an item by asking
//! whether its action is available, and these actions are handled
//! app-wide, so a `disabled` flag would be overridden (see `view_menu`).
//! Each item keeps its own number, the workspace's position in the
//! manager's order, so its shortcut shows beside it whatever else is open.

use gpui_kit::Action;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use uuid::Uuid;

use crate::app_bootstrap::MinimizeWindow;
use crate::app_bootstrap::OpenCommandCenter;
use crate::app_bootstrap::OpenWorkspaces;
use crate::app_bootstrap::ZoomWindow;
use crate::consts::NUMBERED_SHORTCUTS;
use crate::keymap::*;

/// One open workspace window, as the Window menu lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NumberedWorkspace {
    /// The workspace's position in the manager's order, from zero: which
    /// Select Workspace action, and so which shortcut, the item carries.
    pub(crate) position: usize,
    pub(crate) id:       Uuid,
    pub(crate) name:     String,
}

/// What the Window menu lists and checks, compared against the last one
/// built to decide whether the menu bar needs rebuilding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WindowMenuSnapshot {
    /// The first nine workspaces that have an open window, in the manager's
    /// order.
    pub(crate) workspaces: Vec<NumberedWorkspace>,
    /// The workspace of the window that owns the menu bar, checked in the
    /// list; `None` when no workspace window does.
    pub(crate) current:    Option<Uuid>,
}

/// The first nine workspaces, in the manager's order: the ones Select
/// Workspace 1-9 address.
pub(crate) fn numbered_workspaces(store: &knot_agents::AgentStore) -> Vec<(Uuid, String)> {
    store.workspaces()
         .iter()
         .take(NUMBERED_SHORTCUTS)
         .map(|workspace| (workspace.id, workspace.name.clone()))
         .collect()
}

/// Reads the snapshot: the numbered workspaces in `store` whose window is
/// among `open`, with `current` checked.
pub(crate) fn window_menu_snapshot(store: &knot_agents::AgentStore, open: &[Uuid],
                                   current: Option<Uuid>)
                                   -> WindowMenuSnapshot {
    let workspaces =
        numbered_workspaces(store).into_iter()
                                  .enumerate()
                                  .filter(|(_, (id, _))| open.contains(id))
                                  .map(|(position, (id, name))| NumberedWorkspace { position,
                                                                                    id,
                                                                                    name })
                                  .collect();
    WindowMenuSnapshot { workspaces,
                         current }
}

/// The Window menu.
///
/// Knot's own items first, then a separator, then the list of open windows
/// macOS appends and maintains below them. Without the separator a
/// workspace called "Zoom" is indistinguishable from the Zoom command, and
/// these items shift down every time a window opens. The two openers are
/// their own group: they open windows, the workspace items raise one, and
/// Minimize and Zoom manipulate the focused one.
pub(crate) fn window_menu(snapshot: &WindowMenuSnapshot) -> Menu {
    let mut items = vec![MenuItem::action(knot_core::l10n::t("menu.window.command_center"),
                                          OpenCommandCenter),
                         MenuItem::action(knot_core::l10n::t("menu.window.workspaces"),
                                          OpenWorkspaces),
                         MenuItem::separator(),];
    if !snapshot.workspaces.is_empty() {
        items.extend(snapshot.workspaces
                             .iter()
                             .map(|workspace| workspace_item(workspace, snapshot.current)));
        items.push(MenuItem::separator());
    }
    items.extend([MenuItem::action("Minimize", MinimizeWindow),
                  MenuItem::action("Zoom", ZoomWindow),
                  MenuItem::separator()]);
    Menu::new("Window").items(items)
}

/// `workspace`'s item: its name, on its own Select Workspace action so the
/// shortcut shows beside it, checked when it is `current`.
fn workspace_item(workspace: &NumberedWorkspace, current: Option<Uuid>) -> MenuItem {
    // `position` comes from `numbered_workspaces`, which stops at
    // `NUMBERED_SHORTCUTS`, so it always names one of the nine actions.
    let action = SELECT_WORKSPACE_ACTIONS[workspace.position]();
    MenuItem::Action { name: workspace.name.clone().into(),
                       action,
                       os_action: None,
                       checked: current == Some(workspace.id),
                       disabled: false }
}

const SELECT_WORKSPACE_ACTIONS: [fn() -> Box<dyn Action>; NUMBERED_SHORTCUTS] =
    [|| Box::new(SelectWorkspace1),
     || Box::new(SelectWorkspace2),
     || Box::new(SelectWorkspace3),
     || Box::new(SelectWorkspace4),
     || Box::new(SelectWorkspace5),
     || Box::new(SelectWorkspace6),
     || Box::new(SelectWorkspace7),
     || Box::new(SelectWorkspace8),
     || Box::new(SelectWorkspace9)];
