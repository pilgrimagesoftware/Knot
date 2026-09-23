//! A preference changed while a workspace window is open reaches that window
//! (`openspec/specs/settings-persistence/spec.md`, "A preference change
//! applies to open windows").
//!
//! These were written against the copy-and-refresh model: the window held its
//! own `Settings`, and the test drove `settings_broadcast` and then read the
//! window's copy back to prove the refresh had landed. Neither exists now.
//! The window has no copy, so there is nothing to refresh and nothing to read
//! back - `settings_for_test` is gone with the field it exposed.
//!
//! What is still worth pinning is the observable behaviour rather than the
//! mechanism: a preference written through the settings surface is what an
//! already-open window reads afterwards, in both directions, and writing one
//! does not disturb the roster. `compact_tool_calls` stays the preference
//! under test because it is the one whose absence was reported.
//!
//! Through a real `WorkspaceWindow::open` rather than a stand-in view: the
//! defect was in how a real window got its settings, so a test that did not
//! build one would be asserting against the thing that was missing.
//!
//! Every `Settings` here is rooted at a temporary directory:
//! `WorkspaceWindow::open` persists the roster, and a `Settings::default()`
//! carries no store paths, so it would write over the developer's own
//! workspaces and agents.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::settings_global;
use crate::tests::workspace;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// A store holding one agentless workspace, and that workspace's id. No
/// agents, so no session or adapter subprocess starts.
fn store_with_one_workspace(
    )
    -> (Arc<Mutex<knot_agents::AgentStore>>, Arc<Mutex<knot_messaging::MessageStore>>, Uuid)
{
    let mut store = knot_agents::AgentStore::new();
    let workspace = workspace("Only");
    let id = workspace.id;
    store.add_workspace(workspace);
    (Arc::new(Mutex::new(store)), Arc::new(Mutex::new(knot_messaging::MessageStore::new())), id)
}

/// Writes the preference the way the settings window does: through the shared
/// surface, then out to the preferences document.
fn write_compact_tool_calls(enabled: bool, cx: &mut gpui_kit::App) {
    let installed = settings_global::write(cx, |settings| {
        settings.agent_panel_compact_tool_calls = enabled
    });
    installed.persist_preferences()
             .expect("the settings window should be able to write preferences");
}

#[gpui_kit::test]
fn enabling_compact_tool_calls_reaches_an_open_workspace_window(cx: &mut TestAppContext) {
    let (store, messages, id) = store_with_one_workspace();
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          let mut settings = knot_core::Settings::with_store_root(dir.path());
          settings.agent_panel_compact_tool_calls = false;
          settings.persist_preferences().unwrap();
          settings_global::install(settings, cx);

          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), id, cx);
          assert!(WindowRegistry::workspace_view(WindowKey::Workspace(id), cx).is_some(),
                  "opening a workspace should register its view");
          assert!(!settings_global::read(cx).agent_panel_compact_tool_calls,
                  "the window should start with the preference off - otherwise this test cannot \
                   tell a working read from a lucky default");

          write_compact_tool_calls(true, cx);

          assert!(settings_global::read(cx).agent_panel_compact_tool_calls,
                  "the open window draws from this surface, so this is what it now reads");
          let reloaded =
              knot_core::Settings::load_from_root(dir.path()).expect("the document reloads");
          assert!(reloaded.agent_panel_compact_tool_calls,
                  "and the change reached the document, not only memory");
      });
}

/// The other direction, because "turn compact mode off and subsequent tool
/// calls render as individual cells" is its own scenario in
/// `collapsed-tool-call-summary`.
#[gpui_kit::test]
fn disabling_compact_tool_calls_reaches_an_open_workspace_window(cx: &mut TestAppContext) {
    let (store, messages, id) = store_with_one_workspace();
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          let mut settings = knot_core::Settings::with_store_root(dir.path());
          settings.agent_panel_compact_tool_calls = true;
          settings.persist_preferences().unwrap();
          settings_global::install(settings, cx);

          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), id, cx);

          write_compact_tool_calls(false, cx);

          assert!(!settings_global::read(cx).agent_panel_compact_tool_calls,
                  "turning compact mode off left the open window drawing summaries");
      });
}

/// A preference write must not be a way to lose the roster: the roster lives
/// in a different document from the preferences, and the window is drawing
/// from the same surface the write goes through.
#[gpui_kit::test]
fn a_preference_write_leaves_the_roster_alone(cx: &mut TestAppContext) {
    let (store, messages, id) = store_with_one_workspace();
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          let mut settings = knot_core::Settings::with_store_root(dir.path());
          settings.saved_workspaces = store.lock().saved_workspaces();
          settings_global::install(settings, cx);

          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), id, cx);
          let before = settings_global::read(cx).saved_workspaces.len();
          assert_eq!(before, 1,
                     "the surface should be holding the workspace it was given");

          write_compact_tool_calls(true, cx);

          assert_eq!(settings_global::read(cx).saved_workspaces.len(),
                     before,
                     "a preference write emptied the roster");
      });
}

/// The window holds no settings of its own, which is what makes every
/// assertion above about one surface rather than two that happen to agree.
/// A field reintroduced here would pass those tests and still be #238.
#[gpui_kit::test]
fn a_workspace_window_holds_no_settings_of_its_own(cx: &mut TestAppContext) {
    let (store, messages, id) = store_with_one_workspace();
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);

          // `open` takes no `Settings`: there is none to hand it. That this
          // compiles is the assertion; the run below only confirms a window
          // opened without one works.
          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), id, cx);

          assert!(WindowRegistry::workspace_view(WindowKey::Workspace(id), cx).is_some(),
                  "a window opened without a settings copy should still register");
      });
}
