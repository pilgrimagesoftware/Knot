//! The About window entity and how it is opened: the action handler that owns
//! the single-instance handle, and the state the render tree reads.
//!
//! What it draws lives in `super::pane`.

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Styled;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;

use crate::window_options::about_window_options;

/// Registers the `AboutKnot` handler over the window handle it keeps.
///
/// The handle lives in this closure rather than in `run`, so nothing outside
/// can open a second About window past the single-instance check. `run` and
/// the window tests both register through here, so the tests exercise the
/// wiring the app actually installs.
/// `title_font` is the user's title font family (`Settings::title_font_name`,
/// Manrope by default), which every line but the app name is set in - the app
/// name keeps the app-wide UI font, the same split the workspace window makes.
/// It is snapshotted at registration, as the settings window's own `Settings`
/// clone is.
pub(crate) fn register_about_action(title_font: gpui_kit::SharedString, cx: &mut App) {
    let handle: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    cx.on_action(move |_: &crate::app_bootstrap::AboutKnot, cx| {
          open_about_window(&handle, title_font.clone(), cx);
      });
}

/// Raises the open About window, or opens one.
///
/// Whether the last window is still open is asked by updating it: a closed
/// window fails its update, which is the same test the settings window uses
/// and is why no close observer is needed to clear the handle.
///
/// Unlike the alert dialog this replaces, no `cx.defer` is needed. A menu
/// dispatch runs inside the active window's update, so opening a dialog *on
/// that window* was re-entrant; opening a new window is not.
fn open_about_window(handle: &Rc<RefCell<Option<AnyWindowHandle>>>,
                     title_font: gpui_kit::SharedString, cx: &mut App) {
    if let Some(existing) = *handle.borrow()
       && existing.update(cx, |_, window, _| window.activate_window())
                  .is_ok()
    {
        return;
    }
    match cx.open_window(about_window_options(cx), |window, cx| {
                let view = cx.new(|cx| AboutWindow::new(title_font, cx));
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            }) {
        Ok(window) => *handle.borrow_mut() = Some(window.into()),
        Err(error) => eprintln!("failed to open the About window: {error}"),
    }
}

pub(super) struct AboutWindow {
    /// Focused on first render so the window has a key target for `Escape`.
    /// A window with nothing focused never sees the key event at all.
    pub(super) focus:      gpui_kit::FocusHandle,
    /// See `register_about_action`.
    pub(super) title_font: gpui_kit::SharedString,
}

impl AboutWindow {
    fn new(title_font: gpui_kit::SharedString, cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(),
               title_font }
    }
}
