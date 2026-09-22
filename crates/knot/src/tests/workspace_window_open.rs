//! Opening the same workspace twice raises the first window rather than
//! making a second (`openspec/specs/window-lifecycle`, "A workspace has at
//! most one window").
//!
//! Unlike `window_registry`, which covers the registry's own contract with a
//! stand-in window, these go through `WorkspaceWindow::open` itself - the call
//! every route that opens a workspace reaches, and the one the defect was
//! reported against. The workspaces here hold no agents, so no session or
//! adapter subprocess starts.
//!
//! Every `Settings` here is rooted at a temporary directory. `open` persists
//! the roster through `persist_agents`, and a `Settings` built with
//! `default()` carries no store paths, so it falls back to the platform's real
//! application-data directory - which means a test that opens a window writes
//! over the user's own workspaces and agents. `with_store_root` is the only
//! safe way to construct a `Settings` in a test that reaches a persist.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::tests::workspace;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Settings that persist into `dir` rather than into the user's own
/// application-data directory.
fn sandboxed_settings(dir: &TempDir) -> knot_core::Settings {
    knot_core::Settings::with_store_root(dir.path())
}

/// A store holding one agentless workspace, and that workspace's id.
fn store_with_one_workspace(
    )
    -> (Arc<Mutex<knot_agents::AgentStore>>, Arc<Mutex<knot_messaging::MessageStore>>, uuid::Uuid)
{
    let mut store = knot_agents::AgentStore::new();
    let workspace = workspace("Only");
    let id = workspace.id;
    store.add_workspace(workspace);
    (Arc::new(Mutex::new(store)), Arc::new(Mutex::new(knot_messaging::MessageStore::new())), id)
}

#[gpui_kit::test]
fn opening_the_same_workspace_twice_leaves_one_window(cx: &mut TestAppContext) {
    let (store, messages, id) = store_with_one_workspace();
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          let settings = sandboxed_settings(&dir);

          WorkspaceWindow::open(Arc::clone(&store),
                                Arc::clone(&messages),
                                settings.clone(),
                                id,
                                cx);
          assert_eq!(cx.windows().len(), 1, "the first open should make a window");

          for _ in 0..4 {
              WorkspaceWindow::open(Arc::clone(&store),
                                    Arc::clone(&messages),
                                    settings.clone(),
                                    id,
                                    cx);
          }
          assert_eq!(cx.windows().len(),
                     1,
                     "five requests for one workspace should leave one window");
      });
}

#[gpui_kit::test]
fn two_workspaces_get_two_windows(cx: &mut TestAppContext) {
    let (store, messages, first) = store_with_one_workspace();
    let second = {
        let mut store = store.lock();
        let workspace = workspace("Second");
        let id = workspace.id;
        store.add_workspace(workspace);
        id
    };
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          let settings = sandboxed_settings(&dir);

          WorkspaceWindow::open(Arc::clone(&store),
                                Arc::clone(&messages),
                                settings.clone(),
                                first,
                                cx);
          WorkspaceWindow::open(Arc::clone(&store),
                                Arc::clone(&messages),
                                settings.clone(),
                                second,
                                cx);
          assert_eq!(cx.windows().len(),
                     2,
                     "different workspaces keep their own windows");
      });
}
