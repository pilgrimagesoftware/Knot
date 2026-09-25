//! The menu bar's View menu: what it is built from, and the shape it is
//! built into. Contract: `openspec/specs/app-menu`.
//!
//! Which plain items are *enabled* is decided by macOS asking whether each
//! action is available, so it is asserted against a real window in
//! `workspace_window::shortcuts_tests`, not here.

use gpui_kit::Action;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use uuid::Uuid;

use crate::keymap::*;
use crate::tests::workspace;
use crate::view_menu::OwningWindow;
use crate::view_menu::ViewMenuSnapshot;
use crate::view_menu::view_menu;
use crate::view_menu::view_menu_snapshot;
use crate::workspace_window::WorkspaceViewMode;

/// A store with `workspaces` workspaces, the first holding `agents` agents.
fn store_with(workspaces: usize, agents: usize) -> (knot_agents::AgentStore, Vec<Uuid>) {
    let mut store = knot_agents::AgentStore::new();
    let mut ids = Vec::new();
    for index in 0..workspaces {
        let space = workspace(&format!("Space {index}"));
        ids.push(space.id);
        store.add_workspace(space);
    }
    for index in 0..agents {
        store.create(format!("~/agent-{index}"),
                     knot_agents::CreateOptions { workspace_id: Some(ids[0]),
                                                  ..Default::default() });
    }
    (store, ids)
}

fn owning(workspace_id: Uuid) -> OwningWindow {
    OwningWindow { workspace_id,
                   view_mode: WorkspaceViewMode::Terminal,
                   selected_agent: None }
}

/// Each item's label, separators as `"-"` and submenus by their title.
fn labels(menu: &Menu) -> Vec<String> {
    menu.items
        .iter()
        .map(|item| match item {
            MenuItem::Separator => "-".to_string(),
            MenuItem::Action { name, .. } => name.to_string(),
            MenuItem::Submenu(submenu) => submenu.name.to_string(),
            MenuItem::SystemMenu(_) => unreachable!("no OS submenu here"),
        })
        .collect()
}

fn item<'a>(menu: &'a Menu, key: &str) -> &'a MenuItem {
    let label = knot_core::l10n::t(key);
    menu.items
        .iter()
        .find(|item| match item {
            MenuItem::Action { name, .. } => *name == label,
            MenuItem::Submenu(submenu) => submenu.name == label,
            _ => false,
        })
        .unwrap_or_else(|| panic!("the View menu has no {key} item"))
}

fn submenu<'a>(menu: &'a Menu, key: &str) -> &'a Menu {
    match item(menu, key) {
        MenuItem::Submenu(submenu) => submenu,
        _ => panic!("{key} is not a submenu"),
    }
}

fn action_of(item: &MenuItem) -> &dyn Action {
    match item {
        MenuItem::Action { action, .. } => action.as_ref(),
        _ => panic!("not an action item"),
    }
}

#[test]
fn the_items_are_in_the_specified_order() {
    let t = knot_core::l10n::t;
    assert_eq!(labels(&view_menu(&ViewMenuSnapshot::default())),
               [t("menu.view.dashboard"),
                t("menu.view.pull_requests"),
                "-".to_string(),
                t("menu.view.focus_agent_input"),
                t("menu.view.jump_to_bottom"),
                "-".to_string(),
                t("menu.view.select_agent"),
                t("menu.view.select_workspace")]);
}

/// macOS adds Enter Full Screen itself, and only when the menu has no item it
/// judges equivalent (`app-menu`); none of Knot's may be one.
#[test]
fn the_menu_declares_no_full_screen_item() {
    let menu = view_menu(&ViewMenuSnapshot::default());
    assert!(menu.items
                .iter()
                .all(|item| !matches!(item, MenuItem::SystemMenu(_))
                            && !matches!(item,
                                         MenuItem::Action { os_action: Some(_),
                                                            .. })));
}

#[test]
fn each_item_dispatches_its_shortcuts_action() {
    let menu = view_menu(&ViewMenuSnapshot::default());
    for (key, expected) in [("menu.view.dashboard", &ToggleDashboard as &dyn Action),
                            ("menu.view.pull_requests", &TogglePullRequests),
                            ("menu.view.focus_agent_input", &FocusAgentInput),
                            ("menu.view.jump_to_bottom", &JumpToBottom)]
    {
        assert!(action_of(item(&menu, key)).partial_eq(expected),
                "{key} dispatches the wrong action");
    }
}

#[test]
fn the_showing_panel_is_checked() {
    let (store, ids) = store_with(1, 0);
    let window = OwningWindow { view_mode: WorkspaceViewMode::Dashboard,
                                ..owning(ids[0]) };
    let menu = view_menu(&view_menu_snapshot(&store, Some(window)));
    assert!(item(&menu, "menu.view.dashboard").is_checked());
    assert!(!item(&menu, "menu.view.pull_requests").is_checked());

    let menu = view_menu(&view_menu_snapshot(&store, Some(owning(ids[0]))));
    assert!(!item(&menu, "menu.view.dashboard").is_checked());
    assert!(!item(&menu, "menu.view.pull_requests").is_checked());
}

#[test]
fn no_workspace_window_disables_select_agent_but_not_select_workspace() {
    let (store, _) = store_with(2, 3);
    let menu = view_menu(&view_menu_snapshot(&store, None));
    assert!(item(&menu, "menu.view.select_agent").is_disabled());
    assert!(!item(&menu, "menu.view.select_workspace").is_disabled());
    assert_eq!(submenu(&menu, "menu.view.select_workspace").items.len(), 2);
}

#[test]
fn an_empty_workspace_disables_select_agent() {
    let (store, ids) = store_with(1, 0);
    let menu = view_menu(&view_menu_snapshot(&store, Some(owning(ids[0]))));
    assert!(item(&menu, "menu.view.select_agent").is_disabled());
}

#[test]
fn select_agent_lists_the_sidebar_in_order_on_the_numbered_actions() {
    let (store, ids) = store_with(1, 3);
    let agents = crate::workspace_window::workspace_agent_ids(&store, ids[0]);
    let window = OwningWindow { selected_agent: Some(agents[1]),
                                ..owning(ids[0]) };
    let menu = view_menu(&view_menu_snapshot(&store, Some(window)));
    let agent_menu = submenu(&menu, "menu.view.select_agent");

    let names = agents.iter()
                      .map(|id| store.agent(*id).map(|agent| agent.name.clone()))
                      .collect::<Option<Vec<_>>>()
                      .expect("every sidebar agent exists");
    assert_eq!(labels(agent_menu), names);
    for (item, expected) in
        agent_menu.items
                  .iter()
                  .zip([&SelectAgent1 as &dyn Action, &SelectAgent2, &SelectAgent3])
    {
        assert!(action_of(item).partial_eq(expected));
    }
    let checked = agent_menu.items
                            .iter()
                            .map(MenuItem::is_checked)
                            .collect::<Vec<_>>();
    assert_eq!(checked,
               [false, true, false],
               "the selected agent is checked");
}

#[test]
fn the_submenus_stop_at_nine() {
    let (store, ids) = store_with(12, 12);
    let menu = view_menu(&view_menu_snapshot(&store, Some(owning(ids[0]))));
    assert_eq!(submenu(&menu, "menu.view.select_agent").items.len(), 9);
    let workspaces = submenu(&menu, "menu.view.select_workspace");
    assert_eq!(workspaces.items.len(), 9);
    assert!(action_of(&workspaces.items[8]).partial_eq(&SelectWorkspace9));
    assert!(workspaces.items[0].is_checked(),
            "the owning window's workspace is checked");
}

/// The comparison the menu bar's rebuild hangs off: every change the View
/// menu shows makes a different snapshot, and nothing else does.
#[test]
fn the_snapshot_changes_with_what_the_menu_shows() {
    let (mut store, ids) = store_with(2, 2);
    let agents = crate::workspace_window::workspace_agent_ids(&store, ids[0]);
    let base = view_menu_snapshot(&store, Some(owning(ids[0])));
    assert_eq!(view_menu_snapshot(&store, Some(owning(ids[0]))), base);

    let selected = OwningWindow { selected_agent: Some(agents[0]),
                                  ..owning(ids[0]) };
    assert_ne!(view_menu_snapshot(&store, Some(selected)), base);
    let panel = OwningWindow { view_mode: WorkspaceViewMode::PullRequests,
                               ..owning(ids[0]) };
    assert_ne!(view_menu_snapshot(&store, Some(panel)), base);

    store.create("~/agent-new".to_string(),
                 knot_agents::CreateOptions { workspace_id: Some(ids[0]),
                                              ..Default::default() });
    let added = view_menu_snapshot(&store, Some(owning(ids[0])));
    assert_ne!(added, base, "an added agent");

    assert!(store.rename_workspace(ids[1], "Backend"));
    assert_ne!(view_menu_snapshot(&store, Some(owning(ids[0]))),
               added,
               "a renamed workspace");
}
