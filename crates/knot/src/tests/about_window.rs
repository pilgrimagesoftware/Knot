//! The application menu's "About Knot" item opens the About window, once
//! (`openspec/specs/about-ui`).
//!
//! macOS routes a menu item through `App::dispatch_action`, which runs the
//! handler *inside* `active_window.update(...)`.
//! `TestAppContext::dispatch_action` dispatches through that same window
//! update, which is what makes these tests cover the menu path rather than a
//! straight call to the handler - the alert dialog this window replaced was
//! refused outright when opened that way, and looked like a dead menu item.
//!
//! The handler under test is registered by
//! `about_window::register_about_action`, the same call `run` makes, so the
//! window handle these assertions exercise is the one the app installs.

use gpui_kit::component::Root;
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, IntoElement, Render, TestAppContext, Window,
    WindowOptions, div,
};

use crate::about_window::register_about_action;
use crate::app_bootstrap::AboutKnot;

/// A root view with no content of its own: these assertions count windows,
/// they do not read a rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The active window the menu dispatches through, plus the registered
/// handler.
fn app_with_one_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| {
          gpui_kit::init(cx);
          register_about_action(knot_core::Settings::default().title_font_name.into(), cx);
          let window = cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window");
          window.into()
      })
}

fn window_count(cx: &mut TestAppContext) -> usize {
    cx.update(|cx| cx.windows().len())
}

#[gpui_kit::test]
fn the_about_window_opens_when_the_menu_dispatches_into_the_active_window(cx: &mut TestAppContext) {
    let handle = app_with_one_window(cx);
    let before = window_count(cx);

    cx.dispatch_action(handle, AboutKnot);
    cx.run_until_parked();

    assert_eq!(window_count(cx),
               before + 1,
               "About Knot dispatched from the menu opened no window");
}

#[gpui_kit::test]
fn choosing_about_twice_does_not_open_a_second_window(cx: &mut TestAppContext) {
    let handle = app_with_one_window(cx);

    cx.dispatch_action(handle, AboutKnot);
    cx.run_until_parked();
    let after_first = window_count(cx);

    cx.dispatch_action(handle, AboutKnot);
    cx.run_until_parked();

    assert_eq!(window_count(cx),
               after_first,
               "choosing About Knot again opened a second window instead of raising the open one");
}

#[gpui_kit::test]
fn about_opens_again_after_its_window_is_closed(cx: &mut TestAppContext) {
    let handle = app_with_one_window(cx);
    let before = window_count(cx);

    cx.dispatch_action(handle, AboutKnot);
    cx.run_until_parked();

    // The About window is the one that was not there before.
    let about = cx.update(|cx| {
                      *cx.windows()
                         .iter()
                         .find(|window| **window != handle)
                         .expect("the About window is not among the app's windows")
                  });
    about.update(cx, |_, window, _| window.remove_window())
         .expect("the About window went away before it could be closed");
    cx.run_until_parked();
    assert_eq!(window_count(cx),
               before,
               "closing the About window left it open");

    cx.dispatch_action(handle, AboutKnot);
    cx.run_until_parked();

    assert_eq!(window_count(cx),
               before + 1,
               "About Knot did not reopen after its window was closed");
}
