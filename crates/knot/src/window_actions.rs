//! File > Close Window and Window > Minimize / Zoom, which act on whichever
//! window is focused.
//!
//! Contract: `openspec/specs/app-menu/spec.md`.
//!
//! Registered app-wide rather than per window so every window - workspace,
//! manager, settings, the dialogs - answers them the same way the stoplight
//! buttons do, without each window having to opt in.

use gpui_kit::App;
use gpui_kit::Window;

use crate::app_bootstrap::CloseWindow;
use crate::app_bootstrap::MinimizeWindow;
use crate::app_bootstrap::ZoomWindow;

/// Registers the handlers that make the three items enabled and answer.
pub(crate) fn register_focused_window_actions(cx: &mut App) {
    cx.on_action(|_: &CloseWindow, cx| on_active_window(cx, Window::remove_window));
    cx.on_action(|_: &MinimizeWindow, cx| on_active_window(cx, |window| window.minimize_window()));
    cx.on_action(|_: &ZoomWindow, cx| on_active_window(cx, |window| window.zoom_window()));
}

/// Runs `act` on the focused window, if there is one.
///
/// Deferred because both the menu and the key binding dispatch from inside
/// the active window's own update, and gpui refuses a re-entrant
/// `window.update` on that same window (`knot-ui-conventions`, "Dialogs from
/// menu actions").
fn on_active_window(cx: &mut App, act: impl FnOnce(&mut Window) + 'static) {
    let Some(handle) = cx.active_window()
    else {
        return;
    };
    cx.defer(move |cx| {
          // A window closed between dispatch and the deferred run is
          // simply not there to act on.
          let _ = handle.update(cx, |_, window, _| act(window));
      });
}
