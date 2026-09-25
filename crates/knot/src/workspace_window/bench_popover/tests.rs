//! The bench popover's rows and its remove path, and that it reads the bench
//! when it draws rather than from a snapshot.
//!
//! Settings are rooted in a temporary directory: the window persists the
//! roster, and a `Settings::default()` writes over the user's own.

use std::sync::Arc;

use gpui_kit::{Entity, TestAppContext};
use knot_core::BenchAgent;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use super::*;
use crate::settings_global;
use crate::window_registry::{WindowKey, WindowRegistry};

#[test]
fn rows_follow_bench_order_and_show_the_folder_name() {
    let first = BenchAgent::new(Uuid::new_v4(), "Reviewer", Some("🦀".into()), "/src/knot");
    let second = BenchAgent::new(Uuid::new_v4(), "Tester", None, "/src/other/");

    let rows = bench_rows(&[first.clone(), second.clone()]);

    assert_eq!(rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
               ["Reviewer", "Tester"]);
    assert_eq!(rows[0].avatar, "🦀");
    assert_eq!(rows[0].folder_name, "knot");
    assert_eq!(rows[1].folder_name, "other");
    assert_eq!(rows[0].id, first.id);
}

fn open_window(cx: &mut TestAppContext) -> (Entity<WorkspaceWindow>, TempDir) {
    let mut store = knot_agents::AgentStore::new();
    let space = knot_core::Workspace { id:        Uuid::new_v4(),
                                       name:      "Only".to_string(),
                                       color_hex: "#123456".to_string(),
                                       agent_ids: Vec::new(), };
    let workspace = space.id;
    store.add_workspace(space);
    store.set_current_workspace(workspace);
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), workspace, cx);
      });
    let view = cx.update(|cx| WindowRegistry::workspace_view(WindowKey::Workspace(workspace), cx))
                 .expect("opening a workspace registers its view");
    (view, dir)
}

/// Removing an entry takes it off the bench, deploys nothing, and leaves the
/// popover open on the list without it.
#[gpui_kit::test]
fn removing_an_entry_keeps_the_popover_open_and_deploys_nothing(cx: &mut TestAppContext) {
    let (view, _dir) = open_window(cx);
    let keep = BenchAgent::new(Uuid::new_v4(), "Keep", None, "/keep");
    let drop = BenchAgent::new(Uuid::new_v4(), "Drop", None, "/drop");
    let drop_id = drop.id;
    cx.update(|cx| {
          settings_global::write_persisting(cx, |settings| {
              settings.add_bench_agent(keep.clone())?;
              settings.add_bench_agent(drop.clone())
          }).unwrap();
      });

    cx.update(|cx| {
          view.update(cx, |view, cx| {
                  view.bench_popover_open = true;
                  view.remove_bench_entry(drop_id, cx);
                  assert!(view.bench_popover_open);
                  assert!(view.store.lock().agents().is_empty());
              });
          let names = settings_global::read(cx).bench_agents
                                               .iter()
                                               .map(|entry| entry.name.clone())
                                               .collect::<Vec<_>>();
          assert_eq!(names, ["Keep"]);
      });
}

/// The popover draws from the live surface, so an entry saved after the
/// window opened - from another window, say - is listed without anything
/// telling this window about it.
#[gpui_kit::test]
fn an_entry_saved_elsewhere_is_listed_on_the_next_draw(cx: &mut TestAppContext) {
    let (view, _dir) = open_window(cx);
    let entry = BenchAgent::new(Uuid::new_v4(), "Late", None, "/late");
    cx.update(|cx| {
          settings_global::write_persisting(cx, |settings| settings.add_bench_agent(entry.clone()))
              .unwrap();
      });

    cx.update(|cx| {
          let _ = &view;
          let rows = bench_rows(&settings_global::read(cx).bench_agents);
          assert_eq!(rows.len(), 1);
          assert_eq!(rows[0].name, "Late");
      });
}
