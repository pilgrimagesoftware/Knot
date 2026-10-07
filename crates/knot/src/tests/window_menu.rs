//! The menu bar's Window menu: its items, and the open workspace windows it
//! lists on their Select Workspace shortcuts (#543). Contract:
//! `openspec/specs/app-menu`.

use gpui_kit::Action;
use gpui_kit::Menu;
use gpui_kit::MenuItem;
use uuid::Uuid;

use crate::keymap::*;
use crate::menu_bar::MenuBarState;
use crate::menu_bar::refresh_menu_bar_workspaces;
use crate::tests::window_registry::open_blank;
use crate::tests::workspace;
use crate::window_menu::WindowMenuSnapshot;
use crate::window_menu::window_menu;
use crate::window_menu::window_menu_snapshot;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::window_registry::activate_or_open;

/// A store with `count` workspaces, named "Space 0", "Space 1", ...
fn store_with(count: usize) -> (knot_agents::AgentStore, Vec<Uuid>) {
    let mut store = knot_agents::AgentStore::new();
    let mut ids = Vec::new();
    for index in 0..count {
        let space = workspace(&format!("Space {index}"));
        ids.push(space.id);
        store.add_workspace(space);
    }
    (store, ids)
}

/// Each item's label, separators as `"-"`.
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

/// The item labelled `name`.
fn named<'a>(menu: &'a Menu, name: &str) -> &'a MenuItem {
    menu.items
        .iter()
        .find(|item| matches!(item, MenuItem::Action { name: label, .. } if *label == name))
        .unwrap_or_else(|| panic!("the Window menu has no {name} item"))
}

fn action_of(item: &MenuItem) -> &dyn Action {
    match item {
        MenuItem::Action { action, .. } => action.as_ref(),
        _ => panic!("not an action item"),
    }
}

#[test]
fn with_no_workspace_window_open_the_menu_lists_none() {
    let t = knot_core::l10n::t;
    assert_eq!(labels(&window_menu(&WindowMenuSnapshot::default())),
               [t("menu.window.command_center"),
                t("menu.window.workspaces"),
                "-".to_string(),
                "Minimize".to_string(),
                "Zoom".to_string(),
                "-".to_string()]);
}

/// Only open windows are listed, in the manager's order, as their own group
/// between the openers and the window commands.
#[test]
fn the_open_workspace_windows_are_listed_between_the_openers_and_the_commands() {
    let t = knot_core::l10n::t;
    let (store, ids) = store_with(3);

    let menu = window_menu(&window_menu_snapshot(&store, &[ids[2], ids[0]], None));

    assert_eq!(labels(&menu),
               [t("menu.window.command_center"),
                t("menu.window.workspaces"),
                "-".to_string(),
                "Space 0".to_string(),
                "Space 2".to_string(),
                "-".to_string(),
                "Minimize".to_string(),
                "Zoom".to_string(),
                "-".to_string()]);
}

/// An item keeps its own shortcut however many workspaces before it are
/// closed: the third workspace is ⌥⌘3 whether or not the second is open.
#[test]
fn each_item_carries_its_own_workspaces_number() {
    let (store, ids) = store_with(3);

    let menu = window_menu(&window_menu_snapshot(&store, &[ids[0], ids[2]], None));

    assert!(action_of(named(&menu, "Space 0")).partial_eq(&SelectWorkspace1));
    assert!(action_of(named(&menu, "Space 2")).partial_eq(&SelectWorkspace3));
}

#[test]
fn the_owning_windows_workspace_is_checked() {
    let (store, ids) = store_with(2);

    let menu = window_menu(&window_menu_snapshot(&store, &ids, Some(ids[1])));

    assert!(!named(&menu, "Space 0").is_checked());
    assert!(named(&menu, "Space 1").is_checked());
}

/// Past the ninth, a workspace has no shortcut to put an item on.
#[test]
fn the_list_stops_at_the_ninth_workspace() {
    let (store, ids) = store_with(12);

    let snapshot = window_menu_snapshot(&store, &ids, None);

    assert_eq!(snapshot.workspaces.len(), 9);
    assert!(snapshot.workspaces
                    .iter()
                    .all(|workspace| workspace.position < 9));
}

/// The comparison the menu bar's rebuild hangs off.
#[test]
fn the_snapshot_changes_with_what_the_menu_shows() {
    let (mut store, ids) = store_with(2);
    let base = window_menu_snapshot(&store, &[ids[0]], None);
    assert_eq!(window_menu_snapshot(&store, &[ids[0]], None), base);

    assert_ne!(window_menu_snapshot(&store, &ids, None),
               base,
               "a window opened");
    assert_ne!(window_menu_snapshot(&store, &[ids[0]], Some(ids[0])),
               base,
               "the owning window changed");
    assert!(store.rename_workspace(ids[0], "Backend"));
    assert_ne!(window_menu_snapshot(&store, &[ids[0]], None),
               base,
               "an open workspace renamed");
}

/// The workspace manager has no poll, and while it is focused no workspace
/// window owns the bar, so its own changes have to reach the menu directly.
#[gpui_kit::test]
fn a_workspace_change_reaches_the_menu_bar(cx: &mut gpui_kit::TestAppContext) {
    let (store, ids) = store_with(2);
    let store = parking_lot::Mutex::new(store);
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          activate_or_open(WindowKey::Workspace(ids[1]), cx, open_blank);
          cx.set_global(MenuBarState::default());

          refresh_menu_bar_workspaces(&store, cx);
          let listed = names(cx);
          assert_eq!(listed,
                     ["Space 1"],
                     "the one open workspace window is listed");

          assert!(store.lock().rename_workspace(ids[1], "Backend"));
          refresh_menu_bar_workspaces(&store, cx);
          assert_eq!(names(cx), ["Backend"]);
      });
}

/// What the installed Window menu snapshot lists, by name.
fn names(cx: &gpui_kit::App) -> Vec<String> {
    cx.global::<MenuBarState>()
      .snapshot
      .window
      .workspaces
      .iter()
      .map(|workspace| workspace.name.clone())
      .collect()
}
