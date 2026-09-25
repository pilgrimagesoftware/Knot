//! The configurable shortcuts in the real keymap (`keybindings`): their
//! defaults reach their actions, a rebinding moves the chord rather than
//! adding one, and Select workspace N opens or raises without duplicating.
//!
//! Every `Settings` here is rooted at a temporary directory:
//! `install_actions_and_keys` and `WorkspaceWindow::open` reach a persist,
//! and a `Settings::default()` would write over the developer's own store.

use std::sync::Arc;

use gpui_kit::{Action, App, Keystroke, TestAppContext};
use knot_core::ShortcutModifiers;
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::app_bootstrap::OpenCommandCenter;
use crate::app_bootstrap::Quit;
use crate::app_bootstrap::install_actions_and_keys;
use crate::keymap::*;
use crate::window_registry::WindowRegistry;

fn install(settings: knot_core::Settings, cx: &mut App) {
    gpui_kit::init(cx);
    install_actions_and_keys(&settings,
                             Arc::new(Mutex::new(knot_agents::AgentStore::new())),
                             cx);
}

/// The action the highest-precedence binding for `keystroke` invokes.
fn top_action_for(cx: &App, keystroke: &str) -> Option<Box<dyn Action>> {
    let keystroke = Keystroke::parse(keystroke).expect("the test named an unparsable keystroke");
    cx.all_bindings_for_input(&[keystroke])
      .first()
      .map(|binding| binding.action().boxed_clone())
}

#[track_caller]
fn assert_bound(cx: &App, keystroke: &str, expected: &dyn Action) {
    let actual = top_action_for(cx, keystroke);
    assert!(actual.as_ref()
                  .is_some_and(|action| action.partial_eq(expected)),
            "{keystroke} should invoke {}, not {:?}",
            expected.name(),
            actual.map(|action| action.name().to_string()));
}

#[track_caller]
fn assert_not_bound_to(cx: &App, keystroke: &str, unexpected: &dyn Action) {
    let actual = top_action_for(cx, keystroke);
    assert!(!actual.as_ref()
                   .is_some_and(|action| action.partial_eq(unexpected)),
            "{keystroke} should no longer invoke {}",
            unexpected.name());
}

#[gpui_kit::test]
fn the_defaults_reach_their_actions(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          install(knot_core::Settings::with_store_root(dir.path()), cx);
          assert_bound(cx, "cmd-1", &SelectWorkspace1);
          assert_bound(cx, "cmd-3", &SelectWorkspace3);
          assert_bound(cx, "cmd-9", &SelectWorkspace9);
          assert_bound(cx, "cmd-alt-2", &SelectAgent2);
          assert_bound(cx, "cmd-l", &FocusAgentInput);
          assert_bound(cx, "cmd-alt-o", &ToggleDashboard);
          assert_bound(cx, "cmd-alt-p", &TogglePullRequests);
          assert_bound(cx, "cmd-alt-0", &OpenCommandCenter);
          assert_bound(cx, "ctrl-cmd-down", &JumpToBottom);
      });
}

/// The menu bar reads a key equivalent through `bindings_for_action`, so a
/// rebinding that left the old chord visible there would show the wrong key
/// beside Window > Command Center even though the new one works.
#[gpui_kit::test]
fn a_rebinding_moves_the_chord_and_back(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          install(knot_core::Settings::with_store_root(dir.path()), cx);
          let custom = Chord::parse("ctrl-cmd-k").expect("a valid chord");
          let rebound =
              Resolved::defaults().with_chord(Shortcut::OpenCommandCenter, custom.clone())
                                  .with_modifiers(Shortcut::SelectAgent,
                                                  ShortcutModifiers { command: true,
                                                                      control: true,
                                                                      ..Default::default() });
          apply(&rebound, cx);

          assert_bound(cx, "ctrl-cmd-k", &OpenCommandCenter);
          assert_not_bound_to(cx, "cmd-alt-0", &OpenCommandCenter);
          assert_bound(cx, "ctrl-cmd-2", &SelectAgent2);
          assert_not_bound_to(cx, "cmd-alt-2", &SelectAgent2);
          let shown: Vec<Chord> = cx.key_bindings()
                                    .borrow()
                                    .bindings_for_action(&OpenCommandCenter)
                                    .filter_map(Chord::from_binding)
                                    .collect();
          assert_eq!(shown, [custom], "the menu would show a stale key");

          apply(&Resolved::defaults(), cx);
          assert_bound(cx, "cmd-alt-0", &OpenCommandCenter);
          assert_not_bound_to(cx, "ctrl-cmd-k", &OpenCommandCenter);
          assert_bound(cx, "cmd-alt-2", &SelectAgent2);
      });
}

#[gpui_kit::test]
fn a_stored_binding_that_takes_a_fixed_key_is_ignored_at_launch(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let mut settings = knot_core::Settings::with_store_root(dir.path());
    settings.keybindings.toggle_dashboard = Some("cmd-q".into());
    cx.update(|cx| {
          install(settings, cx);
          assert_bound(cx, "cmd-q", &Quit);
          assert_bound(cx, "cmd-alt-o", &ToggleDashboard);
      });
}

#[gpui_kit::test]
fn selecting_a_workspace_opens_it_once_and_ignores_a_missing_one(cx: &mut TestAppContext) {
    let mut store = knot_agents::AgentStore::new();
    store.add_workspace(crate::tests::workspace("First"));
    store.add_workspace(crate::tests::workspace("Second"));
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          register_global_handlers(Arc::clone(&store), Arc::clone(&messages), cx);
      });

    cx.update(|cx| cx.dispatch_action(&SelectWorkspace2));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.windows().len()), 1, "workspace 2 opens");

    cx.update(|cx| cx.dispatch_action(&SelectWorkspace2));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.windows().len()),
               1,
               "a second press raises, not reopens");

    cx.update(|cx| cx.dispatch_action(&SelectWorkspace5));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.windows().len()),
               1,
               "there is no fifth workspace");

    cx.update(|cx| cx.dispatch_action(&SelectWorkspace1));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.windows().len()),
               2,
               "workspace 1 gets its own window");
}
