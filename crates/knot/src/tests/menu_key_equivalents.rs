//! The shortcuts the menu bar shows beside its standard items, per
//! `app-menu`'s spec.
//!
//! A menu item's key equivalent is looked up by its action, so these tests
//! assert the binding rather than the drawn menu: if the keystroke resolves
//! to the item's action, macOS draws it beside the label.
//!
//! The Edit menu's five items are the reason this file drives a real
//! keymap rather than reading a table. They point at gpui's own text
//! actions, which are bound by the toolkit and not by us, so what pins them
//! is that `cmd-c` still resolves to gpui's `Copy` after our bindings are
//! installed - a knot binding on the same key would out-rank it and break
//! copying in every text field.

use gpui_kit::Action;
use gpui_kit::App;
use gpui_kit::Keystroke;
use gpui_kit::TestAppContext;
use gpui_kit::base::input;

use crate::agent_menu::AgentMenuDuplicateAgent;
use crate::agent_menu::AgentMenuForkAgent;
use crate::agent_menu::AgentMenuNewShellCompanion;
use crate::agent_menu::AgentMenuRemoveAgent;
use crate::agent_menu::AgentMenuRestartAgent;
use crate::agent_menu::agent_menu_key_bindings;
use crate::app_bootstrap::CloseWindow;
use crate::app_bootstrap::EnterFullScreen;
use crate::app_bootstrap::HideApp;
use crate::app_bootstrap::HideOthers;
use crate::app_bootstrap::KnotHelp;
use crate::app_bootstrap::MinimizeWindow;
use crate::app_bootstrap::NewWorkspace;
use crate::app_bootstrap::OpenSettings;
use crate::app_bootstrap::Quit;
use crate::app_bootstrap::install_actions_and_keys;

/// An app with the real key bindings installed, over a throwaway settings
/// file so nothing here touches the developer's own configuration.
fn app_with_bindings(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("failed to make a temp settings directory");
    let settings = knot_core::Settings::with_store_root(dir.path());
    // The directory must outlive the test; leaking the handle keeps the
    // path valid and leaves the files for the OS to reap.
    std::mem::forget(dir);

    let store = std::sync::Arc::new(parking_lot::Mutex::new(knot_agents::AgentStore::new()));
    cx.update(|cx| {
          gpui_kit::init(cx);
          install_actions_and_keys(&settings, store, cx);
      });
}

/// The action the highest-precedence binding for `keystroke` invokes.
///
/// `all_bindings_for_input` ignores key contexts and returns matches in
/// precedence order, which is what lets this ask "who owns this key" of the
/// whole app rather than of one focused element.
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

/// Every standard menu item macOS gives a key equivalent has one here,
/// whether or not the behavior behind it is wired up yet. An item whose
/// shortcut is missing reads as one the app does not offer at all.
#[gpui_kit::test]
fn standard_menu_items_carry_their_platform_shortcut(cx: &mut TestAppContext) {
    app_with_bindings(cx);
    cx.update(|cx| {
          assert_bound(cx, "cmd-q", &Quit);
          assert_bound(cx, "cmd-,", &OpenSettings);
          assert_bound(cx, "cmd-h", &HideApp);
          assert_bound(cx, "cmd-alt-h", &HideOthers);
          assert_bound(cx, "cmd-n", &NewWorkspace);
          assert_bound(cx, "cmd-w", &CloseWindow);
          assert_bound(cx, "cmd-ctrl-f", &EnterFullScreen);
          assert_bound(cx, "cmd-m", &MinimizeWindow);
          assert_bound(cx, "cmd-shift-/", &KnotHelp);
      });
}

/// The Agents menu's items are Knot's own, so their keys come from the
/// Swift reference rather than from macOS. Four of the reference's carry
/// over; Close Agent's cmd-w does not, because Close Window has that key
/// here, and this asserts Remove Agent is left without one rather than
/// quietly given the destructive half of a very common keystroke.
#[gpui_kit::test]
fn the_agents_menu_carries_the_reference_shortcuts(cx: &mut TestAppContext) {
    app_with_bindings(cx);
    cx.update(|cx| {
          assert_bound(cx, "cmd-shift-s", &AgentMenuNewShellCompanion);
          assert_bound(cx, "cmd-f", &AgentMenuForkAgent);
          assert_bound(cx, "cmd-d", &AgentMenuDuplicateAgent);
          assert_bound(cx, "cmd-r", &AgentMenuRestartAgent);
          assert_bound(cx, "cmd-w", &CloseWindow);
      });
    assert!(!agent_menu_key_bindings().iter().any(|binding| {
                                                 binding.action().partial_eq(&AgentMenuRemoveAgent)
                                             }),
            "Remove Agent must have no shortcut: cmd-w is Close Window here");
}

/// The Edit menu shows the standard text shortcuts because it points at
/// gpui's own text actions. Were it to point at placeholders of ours, these
/// keys would resolve to those instead - and, being context-less and added
/// later, ours would win the tie and cut, copy, paste and undo would stop
/// working in every text field.
#[gpui_kit::test]
fn the_edit_menu_leaves_the_text_keys_with_gpui(cx: &mut TestAppContext) {
    app_with_bindings(cx);
    cx.update(|cx| {
          assert_bound(cx, "cmd-z", &input::Undo);
          assert_bound(cx, "cmd-shift-z", &input::Redo);
          assert_bound(cx, "cmd-x", &input::Cut);
          assert_bound(cx, "cmd-c", &input::Copy);
          assert_bound(cx, "cmd-v", &input::Paste);
      });
}
