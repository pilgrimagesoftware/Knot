//! The application menu's "About Knot" item opens its dialog.
//!
//! macOS routes a menu item through `App::dispatch_action`, which runs the
//! handler *inside* `active_window.update(...)`. A handler that then opens
//! its dialog with a second `window.update` on that same window is
//! re-entrant, and gpui refuses it with the same "window not found" it
//! reports for a closed window - so the menu item looked dead.
//! `TestAppContext::dispatch_action` dispatches through the same window
//! update, which is what makes this a regression test rather than a
//! straight call to the handler.

use gpui_kit::component::{Root, WindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, IntoElement, Render, TestAppContext, Window,
    WindowOptions, div,
};

use crate::app_bootstrap::{AboutKnot, about_knot};

/// A root view with no content of its own: the assertion reads `Root`'s
/// dialog stack, not the rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[gpui_kit::test]
fn the_about_dialog_opens_when_the_menu_dispatches_into_the_active_window(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.on_action(about_knot);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window")
                   });

    let handle: AnyWindowHandle = window.into();
    cx.dispatch_action(handle, AboutKnot);
    cx.run_until_parked();

    // Through the untyped handle: the typed `WindowHandle<Root>::update`
    // leases the `Root` entity, and `has_active_dialog` reads it.
    let opened = handle.update(cx, |_, window, cx| window.has_active_dialog(cx))
                       .expect("the test window went away");
    assert!(opened,
            "About Knot dispatched from the menu opened no dialog");
}
