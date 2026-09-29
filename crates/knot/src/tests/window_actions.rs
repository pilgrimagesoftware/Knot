//! File > Close Window and Window > Minimize / Zoom answer for whichever
//! window is focused (`openspec/specs/app-menu`, issue #513).
//!
//! They were placeholders with a shortcut and no handler, so AppKit drew them
//! disabled and the stoplight button was the only way to close a window.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::Render;
use gpui_kit::TestAppContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::div;

use crate::app_bootstrap::CloseWindow;
use crate::app_bootstrap::MinimizeWindow;
use crate::app_bootstrap::ZoomWindow;
use crate::window_actions::register_focused_window_actions;

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// An action nothing handles is one AppKit draws disabled, which is the
/// defect: each of these has to have a handler.
#[gpui_kit::test]
fn the_window_items_are_enabled(cx: &mut TestAppContext) {
    cx.update(|cx| {
          register_focused_window_actions(cx);
          assert!(cx.is_action_available(&CloseWindow),
                  "Close Window is disabled");
          assert!(cx.is_action_available(&MinimizeWindow),
                  "Minimize is disabled");
          assert!(cx.is_action_available(&ZoomWindow), "Zoom is disabled");
      });
}

#[gpui_kit::test]
fn close_window_closes_the_focused_window(cx: &mut TestAppContext) {
    let window = cx.update(|cx| {
                       register_focused_window_actions(cx);
                       cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Blank))
                         .expect("failed to open a window")
                   });
    cx.update(|cx| {
          window.update(cx, |_, window, _| window.activate_window())
                .expect("the window should be open");
      });
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| cx.windows().len()), 1);

    // Dispatched from inside the window's update, as the menu and the key
    // binding both are, so the deferral is what is being exercised.
    cx.update(|cx| {
          window.update(cx, |_, window, cx| {
                    window.dispatch_action(Box::new(CloseWindow), cx)
                })
                .expect("the window should be open");
      });
    cx.run_until_parked();

    assert!(cx.update(|cx| cx.windows().is_empty()),
            "Close Window left the focused window open");
}
