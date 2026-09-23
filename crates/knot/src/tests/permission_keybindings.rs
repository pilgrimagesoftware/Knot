//! The keys behind the inline permission prompt's decision buttons, per
//! `openspec/specs/permission-prompt-ui/spec.md`.
//!
//! Two things have to hold for the prompt to be operable from the keyboard
//! *and* say so, and they can fail independently:
//!
//! - the keystroke has to reach the action, which
//!   `the_decision_keys_reach_their_actions` asserts against the real keymap;
//! - `Kbd` has to find that binding from the action, which is the lookup
//!   `render_permission_prompt` makes per frame to decide what to draw. A
//!   binding installed in a key context `Kbd::global_binding_for_action` does
//!   not search would leave a working shortcut with no hint, so
//!   `the_decision_buttons_find_a_hint_to_draw` asserts the lookup rather than
//!   the keymap.
//!
//! Every `Settings` here is rooted at a temporary directory:
//! `install_actions_and_keys` reaches a persist, and a `Settings::default()`
//! would write over the developer's own workspaces and agents.

use std::sync::Arc;

use gpui_kit::component::Root;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::{
    Action, AnyWindowHandle, App, AppContext, Context, IntoElement, Keystroke, Render,
    TestAppContext, Window, WindowOptions, div,
};
use parking_lot::Mutex;

use crate::app_bootstrap::install_actions_and_keys;
use crate::app_bootstrap::{PanelPermissionAllow, PanelPermissionDeny};

/// A root view with no content: nothing here reads the rendered frame, only
/// the keymap the window resolves against.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A test app with the real key bindings installed and one open window, over
/// a throwaway settings file.
fn app_with_bindings(cx: &mut TestAppContext) -> AnyWindowHandle {
    let dir = tempfile::tempdir().expect("failed to make a temp settings directory");
    let settings = knot_core::Settings::with_store_root(dir.path());
    // The directory must outlive the test; leaking the handle keeps the path
    // valid and leaves the files for the OS to reap.
    std::mem::forget(dir);

    let store = Arc::new(Mutex::new(knot_agents::AgentStore::new()));
    cx.update(|cx| {
          gpui_kit::init(cx);
          install_actions_and_keys(&settings, store, cx);
          cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Blank);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open the test window")
            .into()
      })
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

/// Losing either of these turns the prompt back into a mouse-only control,
/// which is the state issue #194 was raised against.
#[gpui_kit::test]
fn the_decision_keys_reach_their_actions(cx: &mut TestAppContext) {
    app_with_bindings(cx);
    cx.update(|cx| {
          assert_bound(cx, "cmd-shift-a", &PanelPermissionAllow);
          assert_bound(cx, "cmd-shift-d", &PanelPermissionDeny);
      });
}

/// The same bindings, asked for the way the buttons ask: a shortcut that
/// fires but that `Kbd` cannot find stays invisible, which is the failure
/// the hint exists to prevent.
#[gpui_kit::test]
fn the_decision_buttons_find_a_hint_to_draw(cx: &mut TestAppContext) {
    let window = app_with_bindings(cx);
    window.update(cx, |_, window, _| {
              assert!(Kbd::global_binding_for_action(&PanelPermissionAllow, window).is_some(),
                      "the Allow button would draw no hint");
              assert!(Kbd::global_binding_for_action(&PanelPermissionDeny, window).is_some(),
                      "the Deny button would draw no hint");
          })
          .expect("the test window went away");
}
