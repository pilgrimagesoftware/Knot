//! The "scroll to latest" control that floats over a scrolled-up
//! conversation.
//!
//! It sits on top of the message list, so whatever row happens to scroll
//! under it - a collapsed tool call, most often - shares its pixels. GPUI
//! hit-tests every hitbox under the pointer, not just the topmost, so without
//! blocking the click reached that row as well and expanded it (#594).

use gpui_kit::App;
use gpui_kit::ClickEvent;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::div;

/// The control, positioned in the bottom-right corner of a `relative`
/// parent, calling `on_click` when pressed.
///
/// Blocks every mouse interaction with what is behind it except scrolling:
/// a click is the control's alone, but a wheel over it still scrolls the
/// conversation, the way it would a hair to either side.
pub(super) fn scroll_to_latest(on_click: impl Fn(&mut App) + 'static) -> impl IntoElement {
    div().absolute()
         .bottom_3()
         .right_4()
         .block_mouse_except_scroll()
         .debug_selector(|| "panel-scroll-to-latest".into())
         .child(Button::new("panel-scroll-to-bottom").icon(IconName::ChevronDown)
                                                     .tooltip(knot_core::l10n::t("panel.scroll_to_latest"))
                                                     .small()
                                                     .on_click(move |_: &ClickEvent, _, cx| {
                                                         on_click(cx)
                                                     }))
}

#[cfg(test)]
mod tests;
