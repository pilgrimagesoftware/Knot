//! The settings window's Keyboard tab (`keybindings`: "Customizing the
//! shortcuts" and "A customization cannot break other shortcuts"), driven
//! through the pane's own edit paths on a real settings window.
//!
//! Every `Settings` here is rooted at a temporary directory: each accepted
//! edit persists the preferences document, and a `Settings::default()` would
//! write over the developer's own.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::Root;
use gpui_kit::{
    Action, AnyWindowHandle, App, Entity, Keystroke, TestAppContext, VisualTestContext,
};
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::app_bootstrap::install_actions_and_keys;
use crate::keymap::*;
use crate::settings_window::SettingsWindow;
use crate::settings_window::open_settings_window;

struct Fixture {
    window: AnyWindowHandle,
    view:   Entity<SettingsWindow>,
    dir:    TempDir,
}

fn settings_window(cx: &mut TestAppContext) -> Fixture {
    let dir = TempDir::new().expect("a temporary settings root");
    let store = Arc::new(Mutex::new(knot_agents::AgentStore::new()));
    let handle: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    let (window, view) = cx.update(|cx| {
                               gpui_kit::init(cx);
                               let settings = knot_core::Settings::with_store_root(dir.path());
                               crate::settings_global::install(settings.clone(), cx);
                               install_actions_and_keys(&settings, Arc::clone(&store), cx);
                               open_settings_window(&handle, store, cx);
                               let window = handle.borrow().expect("the settings window opened");
                               let view = window.downcast::<Root>()
                                                .expect("the settings window's root is a Root")
                                                .read(cx)
                                                .expect("the settings window is open")
                                                .view()
                                                .clone()
                                                .downcast::<SettingsWindow>()
                                                .expect("the Root holds the settings view");
                               (window, view)
                           });
    Fixture { window, view, dir }
}

fn keystroke(source: &str) -> Keystroke {
    Keystroke::parse(source).expect("the test named an unparsable keystroke")
}

fn record(fixture: &Fixture, shortcut: Shortcut, pressed: &str, cx: &mut TestAppContext) {
    cx.update(|cx| {
          fixture.view.update(cx, |view, cx| {
                          view.start_recording(shortcut, cx);
                          view.finish_recording(&keystroke(pressed), cx);
                      });
      });
}

fn top_action_is(cx: &App, pressed: &str, expected: &dyn Action) -> bool {
    cx.all_bindings_for_input(&[keystroke(pressed)])
      .first()
      .is_some_and(|binding| binding.action().partial_eq(expected))
}

/// Whether some document under the store root now holds `needle`.
fn persisted(dir: &TempDir, needle: &str) -> bool {
    walk(dir.path()).iter()
                    .filter_map(|path| std::fs::read_to_string(path).ok())
                    .any(|text| text.contains(needle))
}

fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        }
        else {
            out.push(path);
        }
    }
    out
}

#[gpui_kit::test]
fn a_recorded_chord_is_stored_and_takes_over_the_shortcut(cx: &mut TestAppContext) {
    let fixture = settings_window(cx);
    record(&fixture, Shortcut::FocusAgentInput, "ctrl-cmd-i", cx);
    cx.update(|cx| {
          assert_eq!(crate::settings_global::read(cx).keybindings
                                                     .focus_input
                                                     .as_deref(),
                     Some("ctrl-cmd-i"));
          assert!(top_action_is(cx, "ctrl-cmd-i", &FocusAgentInput));
          assert!(!top_action_is(cx, "cmd-l", &FocusAgentInput),
                  "the old chord still fires");
      });
    assert!(persisted(&fixture.dir, "ctrl-cmd-i"),
            "the change was not written");
}

#[gpui_kit::test]
fn escape_cancels_a_recording(cx: &mut TestAppContext) {
    let fixture = settings_window(cx);
    record(&fixture, Shortcut::FocusAgentInput, "escape", cx);
    cx.update(|cx| {
          assert_eq!(crate::settings_global::read(cx).keybindings.focus_input,
                     None);
          assert!(top_action_is(cx, "cmd-l", &FocusAgentInput));
          assert_eq!(fixture.view.read(cx).keyboard_state().recording,
                     None,
                     "the recorder stays armed");
      });
}

#[gpui_kit::test]
fn a_conflicting_chord_is_refused_with_the_holders_name(cx: &mut TestAppContext) {
    let fixture = settings_window(cx);
    record(&fixture, Shortcut::ToggleDashboard, "cmd-w", cx);
    cx.update(|cx| {
          let rejection = fixture.view.read(cx).keyboard_state().rejection.clone();
          let (shortcut, message) = rejection.expect("the edit should have been refused");
          assert_eq!(shortcut, Shortcut::ToggleDashboard);
          assert!(message.contains(&knot_core::l10n::t("keymap.fixed.close_window")),
                  "{message}");
          assert_eq!(crate::settings_global::read(cx).keybindings
                                                     .toggle_dashboard,
                     None);
          assert!(top_action_is(cx, "cmd-alt-o", &ToggleDashboard));
      });
}

#[gpui_kit::test]
fn resetting_puts_the_default_back(cx: &mut TestAppContext) {
    let fixture = settings_window(cx);
    record(&fixture, Shortcut::ToggleDashboard, "ctrl-cmd-b", cx);
    cx.update(|cx| {
          fixture.view.update(cx, |view, cx| {
                          view.reset_keybinding(Shortcut::ToggleDashboard, cx)
                      });
          assert_eq!(crate::settings_global::read(cx).keybindings
                                                     .toggle_dashboard,
                     None);
          assert!(top_action_is(cx, "cmd-alt-o", &ToggleDashboard));
          assert!(!top_action_is(cx, "ctrl-cmd-b", &ToggleDashboard));
      });
}

#[gpui_kit::test]
fn toggling_a_family_modifier_moves_all_nine(cx: &mut TestAppContext) {
    let fixture = settings_window(cx);
    cx.update(|cx| {
          // ⌥⌘ to ⌥⌃⌘ ...
          fixture.view.update(cx, |view, cx| {
                          view.toggle_family_modifier(Shortcut::SelectAgent,
                                                      |m| m.control = !m.control,
                                                      cx);
                      });
          assert!(top_action_is(cx, "ctrl-alt-cmd-2", &SelectAgent2));
          assert!(!top_action_is(cx, "alt-cmd-2", &SelectAgent2));
          // ... and dropping ⌥ and ⌘ from that leaves ⌃ alone, which is
          // allowed.
          fixture.view.update(cx, |view, cx| {
                          view.toggle_family_modifier(Shortcut::SelectAgent,
                                                      |m| m.alt = !m.alt,
                                                      cx);
                          view.toggle_family_modifier(Shortcut::SelectAgent,
                                                      |m| m.command = !m.command,
                                                      cx);
                      });
          assert!(top_action_is(cx, "ctrl-9", &SelectAgent9));
      });
}

/// The recorder's interception, end to end: a real key press reaches the
/// armed recorder through the app's keystroke interceptor, not through a
/// call the test makes, and disarms it.
#[gpui_kit::test]
fn an_armed_recorder_takes_the_next_key_press(cx: &mut TestAppContext) {
    let fixture = settings_window(cx);
    cx.update(|cx| {
          fixture.view.update(cx, |view, cx| {
                          view.start_recording(Shortcut::FocusAgentInput, cx)
                      });
      });
    let mut window = VisualTestContext::from_window(fixture.window, cx);
    window.simulate_keystrokes("ctrl-cmd-j");
    window.run_until_parked();
    cx.update(|cx| {
          assert_eq!(crate::settings_global::read(cx).keybindings
                                                     .focus_input
                                                     .as_deref(),
                     Some("ctrl-cmd-j"));
          assert_eq!(fixture.view.read(cx).keyboard_state().recording, None);
      });
}
