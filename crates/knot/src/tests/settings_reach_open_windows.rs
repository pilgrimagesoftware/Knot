//! A preference changed while a workspace window is open reaches that
//! window (`openspec/specs/settings-persistence.md`, "A preference change
//! applies to open windows").
//!
//! Through a real `WorkspaceWindow::open` rather than a stand-in view: the
//! defect was that the window held its own copy of the settings surface, so a
//! test that did not build a real window would be asserting against the very
//! thing that was missing. `compact_tool_calls` is the preference under test
//! because it is the one whose absence was reported - the panel drew per-call
//! cards however the setting was toggled.
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

use crate::settings_broadcast;
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

/// Writes the preferences document the way the settings window does: its own
/// copy of the surface, rooted at the same store, persisted.
fn write_compact_tool_calls(dir: &TempDir, enabled: bool) {
    let mut settings_window_copy = knot_core::Settings::with_store_root(dir.path());
    settings_window_copy.agent_panel_compact_tool_calls = enabled;
    settings_window_copy.persist_preferences()
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

          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), settings, id, cx);

          let view = WindowRegistry::workspace_view(WindowKey::Workspace(id), cx)
              .expect("opening a workspace should register its view");
          assert!(!view.read(cx)
                       .settings_for_test()
                       .agent_panel_compact_tool_calls,
                  "the window should start with the preference off - otherwise this test cannot \
                   tell a working refresh from a lucky default");

          write_compact_tool_calls(&dir, true);
          settings_broadcast::preferences_changed(cx);

          assert!(view.read(cx)
                      .settings_for_test()
                      .agent_panel_compact_tool_calls,
                  "the open window never saw the preference change - this is issue #238");
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

          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), settings, id, cx);

          let view = WindowRegistry::workspace_view(WindowKey::Workspace(id), cx)
              .expect("opening a workspace should register its view");

          write_compact_tool_calls(&dir, false);
          settings_broadcast::preferences_changed(cx);

          assert!(!view.read(cx)
                       .settings_for_test()
                       .agent_panel_compact_tool_calls,
                  "turning compact mode off left the open window drawing summaries");
      });
}

/// A refresh must not be a second way to lose the roster: the window's
/// snapshot carries the workspaces it is drawing, and those live in a
/// different document from the preferences being re-read.
#[gpui_kit::test]
fn a_refresh_leaves_the_windows_roster_alone(cx: &mut TestAppContext) {
    let (store, messages, id) = store_with_one_workspace();
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          // The roster is put on the snapshot here rather than left to the
          // window: `open` persists the roster out of the agent store, it does
          // not read it back onto the copy it was handed. What matters is that
          // the window is holding one when the refresh arrives.
          let mut settings = knot_core::Settings::with_store_root(dir.path());
          settings.saved_workspaces = store.lock().saved_workspaces();
          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), settings, id, cx);

          let view = WindowRegistry::workspace_view(WindowKey::Workspace(id), cx)
              .expect("opening a workspace should register its view");
          let before = view.read(cx).settings_for_test().saved_workspaces.len();
          assert_eq!(before, 1,
                     "the window should be holding the workspace it was given");

          write_compact_tool_calls(&dir, true);
          settings_broadcast::preferences_changed(cx);

          assert_eq!(view.read(cx).settings_for_test().saved_workspaces.len(),
                     before,
                     "the refresh emptied the window's roster snapshot");
      });
}

/// The broadcast walks a registry that outlives the windows in it. A closed
/// window's entry must not be a panic or a resurrection on the next
/// preference change.
#[gpui_kit::test]
fn a_broadcast_with_no_open_windows_does_nothing(cx: &mut TestAppContext) {
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          settings_broadcast::preferences_changed(cx);

          assert!(WindowRegistry::workspace_views(cx).is_empty(),
                  "there were no windows to find");
      });
}
