//! The menu bar's **View** menu: the workspace navigation shortcuts, as items.
//!
//! Contract: `openspec/specs/app-menu/spec.md`. Design:
//! `openspec/changes/view-menu-navigation/design.md`.
//!
//! Every item dispatches its shortcut's own action. That is what puts the
//! current binding beside the label - gpui looks a menu item's key equivalent
//! up by action - and what guarantees the item and the key do the same thing.
//!
//! Enablement is not written into the [`Menu`] for those items, for the
//! reason `agent_menu` gives: macOS asks `App::is_action_available`, which
//! answers from the focused window's dispatch tree and overrides any
//! `disabled` flag. The workspace window registers each shortcut's handler
//! only while it applies (`WorkspaceWindow::with_shortcut_actions`), so the
//! items follow on their own. The two submenus' parents carry no action and
//! are never validated, so their state comes from the snapshot - as do the
//! checkmarks, which AppKit never computes.

use gpui_kit::Action;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use uuid::Uuid;

use crate::consts::NUMBERED_SHORTCUTS;
use crate::keymap::*;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::workspace_agent_ids;

/// What the View menu lists and checks, compared against the last one built
/// to decide whether the menu bar needs rebuilding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ViewMenuSnapshot {
    /// The workspace window that owns the menu bar; `None` when none does.
    pub(crate) window:     Option<WorkspaceViewFacts>,
    /// The first nine workspaces, in the manager's order - listed whether or
    /// not a workspace window owns the bar, since Select workspace N works
    /// from anywhere.
    pub(crate) workspaces: Vec<(Uuid, String)>,
}

/// The owning workspace window's side of [`ViewMenuSnapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceViewFacts {
    pub(crate) workspace_id:   Uuid,
    pub(crate) view_mode:      WorkspaceViewMode,
    /// The first nine sidebar agents, top to bottom.
    pub(crate) agents:         Vec<(Uuid, String)>,
    pub(crate) selected_agent: Option<Uuid>,
}

/// What the owning window shows, as [`view_menu_snapshot`] needs it.
pub(crate) struct OwningWindow {
    pub(crate) workspace_id:   Uuid,
    pub(crate) view_mode:      WorkspaceViewMode,
    pub(crate) selected_agent: Option<Uuid>,
}

/// Reads the snapshot from `store`, for `window` or for no workspace window.
pub(crate) fn view_menu_snapshot(store: &knot_agents::AgentStore, window: Option<OwningWindow>)
                                 -> ViewMenuSnapshot {
    let workspaces = store.workspaces()
                          .iter()
                          .take(NUMBERED_SHORTCUTS)
                          .map(|workspace| (workspace.id, workspace.name.clone()))
                          .collect();
    let window = window.map(|window| WorkspaceViewFacts { workspace_id:   window.workspace_id,
                                                          view_mode:      window.view_mode,
                                                          agents:
                                                              sidebar_agents(store,
                                                                             window.workspace_id),
                                                          selected_agent: window.selected_agent, });
    ViewMenuSnapshot { window, workspaces }
}

/// The first nine agents in `workspace_id`'s sidebar, top to bottom.
fn sidebar_agents(store: &knot_agents::AgentStore, workspace_id: Uuid) -> Vec<(Uuid, String)> {
    workspace_agent_ids(store, workspace_id).into_iter()
                                            .filter_map(|id| {
                                                store.agent(id)
                                                     .map(|agent| (id, agent.name.clone()))
                                            })
                                            .take(NUMBERED_SHORTCUTS)
                                            .collect()
}

/// The View menu. macOS appends Enter Full Screen below these items itself,
/// which is why none of them is a full-screen item (`app-menu`).
pub(crate) fn view_menu(snapshot: &ViewMenuSnapshot) -> Menu {
    let showing = snapshot.window.as_ref().map(|window| window.view_mode);
    Menu::new("View").items([
        MenuItem::action(knot_core::l10n::t("menu.view.dashboard"), ToggleDashboard)
            .checked(showing == Some(WorkspaceViewMode::Dashboard)),
        MenuItem::action(knot_core::l10n::t("menu.view.pull_requests"), TogglePullRequests)
            .checked(showing == Some(WorkspaceViewMode::PullRequests)),
        MenuItem::separator(),
        MenuItem::action(knot_core::l10n::t("menu.view.focus_agent_input"), FocusAgentInput),
        MenuItem::action(knot_core::l10n::t("menu.view.jump_to_bottom"), JumpToBottom),
        MenuItem::separator(),
        select_agent_submenu(snapshot.window.as_ref()),
        select_workspace_submenu(snapshot),
    ])
}

/// The owning window's first nine agents. Disabled with no owning window or
/// no agents: a parent item is enabled by AppKit unless marked otherwise.
fn select_agent_submenu(window: Option<&WorkspaceViewFacts>) -> MenuItem {
    let items =
        window.map(|window| {
                  numbered_items(&window.agents, window.selected_agent, SELECT_AGENT_ACTIONS)
              })
              .unwrap_or_default();
    let disabled = items.is_empty();
    MenuItem::submenu(Menu::new(knot_core::l10n::t("menu.view.select_agent")).items(items))
        .disabled(disabled)
}

fn select_workspace_submenu(snapshot: &ViewMenuSnapshot) -> MenuItem {
    let current = snapshot.window.as_ref().map(|window| window.workspace_id);
    let items = numbered_items(&snapshot.workspaces, current, SELECT_WORKSPACE_ACTIONS);
    let disabled = items.is_empty();
    MenuItem::submenu(Menu::new(knot_core::l10n::t("menu.view.select_workspace")).items(items))
        .disabled(disabled)
}

/// One item per target, the Nth on the Nth action, with `checked` marked.
fn numbered_items(targets: &[(Uuid, String)], checked: Option<Uuid>,
                  actions: [fn() -> Box<dyn Action>; NUMBERED_SHORTCUTS])
                  -> Vec<MenuItem> {
    targets.iter()
           .zip(actions)
           .map(|((id, name), action)| MenuItem::Action { name:      name.clone().into(),
                                                          action:    action(),
                                                          os_action: None,
                                                          checked:   checked == Some(*id),
                                                          disabled:  false, })
           .collect()
}

const SELECT_AGENT_ACTIONS: [fn() -> Box<dyn Action>; NUMBERED_SHORTCUTS] =
    [|| Box::new(SelectAgent1),
     || Box::new(SelectAgent2),
     || Box::new(SelectAgent3),
     || Box::new(SelectAgent4),
     || Box::new(SelectAgent5),
     || Box::new(SelectAgent6),
     || Box::new(SelectAgent7),
     || Box::new(SelectAgent8),
     || Box::new(SelectAgent9)];

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
