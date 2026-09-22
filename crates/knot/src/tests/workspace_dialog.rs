//! The workspace name dialog answers Return and Escape, per
//! `workspace-manager-ui`'s spec.
//!
//! The keys ride on the overlay's bubble-phase `on_key_down`, which only
//! works because the focused `Input` propagates both: gpui-base binds
//! `enter` and `escape` in its `Input` key context, and its handlers call
//! `cx.propagate()` for a single-line field that does not set
//! `clean_on_escape`. That is a property of a dependency rather than of
//! this crate, so these tests drive real keystrokes through a real window
//! instead of calling the two dialog methods directly - a gpui-component
//! upgrade that stopped propagating would break the dialog silently, and
//! this is what would catch it.

use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::component::Root;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::{AppContext, TestAppContext, VisualTestContext, WindowOptions};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::tests::workspace;
use crate::workspace_manager::WorkspaceManager;
use crate::workspace_manager::workspace_name_is_blank;

/// A manager wired to a throwaway settings file, so `persist` writes into
/// the temp directory rather than the developer's real config.
fn manager(
    cx: &mut TestAppContext)
    -> (Arc<Mutex<knot_agents::AgentStore>>, VisualTestContext, Entity<WorkspaceManager>) {
    let dir = tempfile::tempdir().expect("failed to make a temp settings directory");
    let settings = knot_core::Settings::with_store_root(dir.path());
    // The directory must outlive the test; leaking the handle keeps the
    // path valid and leaves the files for the OS to reap.
    std::mem::forget(dir);

    let store = Arc::new(Mutex::new(knot_agents::AgentStore::new()));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));

    let mut created: Option<Entity<WorkspaceManager>> = None;
    let window = {
        let store = Arc::clone(&store);
        let created = &mut created;
        cx.update(|cx| {
              gpui_kit::init(cx);
              cx.open_window(WindowOptions::default(), |window, cx| {
                    let name_input =
                        cx.new(|cx| InputState::new(window, cx).placeholder("Workspace name"));
                    let view = cx.new(|cx| {
                                     let name_subscription =
                                         cx.subscribe(&name_input,
                                                      |_: &mut WorkspaceManager, _, event, cx| {
                                                          if matches!(event, InputEvent::Change) {
                                                              cx.notify();
                                                          }
                                                      });
                                     WorkspaceManager { store,
                                                        messages,
                                                        settings,
                                                        name_input,
                                                        editing_id: None,
                                                        workspace_dialog_id: None,
                                                        show_workspace_dialog: false,
                                                        error: None,
                                                        _name_subscription: name_subscription,
                                                        _mcp_stop: None }
                                 });
                    *created = Some(view.clone());
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("failed to open the test window")
          })
    };

    let manager = created.expect("the window builder never ran");
    let cx = VisualTestContext::from_window(window.into(), cx);
    (store, cx, manager)
}

#[test]
fn a_blank_workspace_name_is_one_with_nothing_but_whitespace_in_it() {
    assert!(workspace_name_is_blank(""), "an empty name is blank");
    assert!(workspace_name_is_blank("   "), "spaces alone are blank");
    assert!(workspace_name_is_blank(" \t\n "),
            "any run of whitespace is blank");
    assert!(!workspace_name_is_blank("Knot"),
            "a typed name is not blank");
    assert!(!workspace_name_is_blank("  Knot  "),
            "surrounding whitespace does not make a name blank");
}

#[gpui_kit::test]
fn escape_closes_the_workspace_dialog_and_creates_nothing(cx: &mut TestAppContext) {
    let (store, mut cx, manager) = manager(cx);

    let before = store.lock().workspaces().len();
    manager.update_in(&mut cx, |manager, window, cx| {
               manager.open_workspace_dialog(None, window, cx);
               manager.name_input.update(cx, |input, cx| {
                                     input.set_value("Discarded", window, cx);
                                 });
           });
    cx.run_until_parked();

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();

    assert!(!manager.read_with(&cx, |manager, _| manager.show_workspace_dialog),
            "escape left the workspace dialog open");
    assert_eq!(store.lock().workspaces().len(),
               before,
               "escape created a workspace");
}

#[gpui_kit::test]
fn return_creates_the_workspace_the_dialog_was_naming(cx: &mut TestAppContext) {
    let (store, mut cx, manager) = manager(cx);

    manager.update_in(&mut cx, |manager, window, cx| {
               manager.open_workspace_dialog(None, window, cx);
               manager.name_input.update(cx, |input, cx| {
                                     input.set_value("  Created  ", window, cx);
                                 });
           });
    cx.run_until_parked();

    cx.simulate_keystrokes("enter");
    cx.run_until_parked();

    assert!(!manager.read_with(&cx, |manager, _| manager.show_workspace_dialog),
            "return left the workspace dialog open");
    let names: Vec<String> = store.lock()
                                  .workspaces()
                                  .iter()
                                  .map(|workspace| workspace.name.clone())
                                  .collect();
    assert!(names.iter().any(|name| name == "Created"),
            "return created no workspace with the trimmed name, only {names:?}");
}

#[gpui_kit::test]
fn return_on_a_blank_name_does_nothing_at_all(cx: &mut TestAppContext) {
    let (store, mut cx, manager) = manager(cx);

    let before = store.lock().workspaces().len();
    manager.update_in(&mut cx, |manager, window, cx| {
               manager.open_workspace_dialog(None, window, cx);
               manager.name_input.update(cx, |input, cx| {
                                     input.set_value("   ", window, cx);
                                 });
           });
    cx.run_until_parked();

    cx.simulate_keystrokes("enter");
    cx.run_until_parked();

    assert!(manager.read_with(&cx, |manager, _| manager.show_workspace_dialog),
            "return on a blank name closed the dialog");
    assert_eq!(store.lock().workspaces().len(),
               before,
               "return on a blank name created a workspace");
    assert!(manager.read_with(&cx, |manager, _| manager.error.is_none()),
            "return on a blank name raised an error message");
}

#[gpui_kit::test]
fn the_same_two_keys_work_when_the_dialog_is_renaming(cx: &mut TestAppContext) {
    let (store, mut cx, manager) = manager(cx);

    let existing = workspace("Before");
    let id = existing.id;
    store.lock().add_workspace(existing);

    // Escape first: the rename is discarded and the old name stands.
    manager.update_in(&mut cx, |manager, window, cx| {
               manager.open_workspace_dialog(Some(id), window, cx);
               manager.name_input.update(cx, |input, cx| {
                                     input.set_value("Discarded", window, cx);
                                 });
           });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(workspace_name(&store, id).as_deref(),
               Some("Before"),
               "escape applied the rename it should have discarded");

    // Then Return: the rename lands, through the same code path.
    manager.update_in(&mut cx, |manager, window, cx| {
               manager.open_workspace_dialog(Some(id), window, cx);
               manager.name_input.update(cx, |input, cx| {
                                     input.set_value("After", window, cx);
                                 });
           });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(workspace_name(&store, id).as_deref(),
               Some("After"),
               "return did not apply the rename");
    assert!(!manager.read_with(&cx, |manager, _| manager.show_workspace_dialog),
            "return left the rename dialog open");
}

fn workspace_name(store: &Arc<Mutex<knot_agents::AgentStore>>, id: Uuid) -> Option<String> {
    store.lock()
         .workspaces()
         .iter()
         .find(|workspace| workspace.id == id)
         .map(|workspace| workspace.name.clone())
}
