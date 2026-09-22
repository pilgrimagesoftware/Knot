//! Control constructors shared by more than one window.
//!
//! These began as associated functions on `SettingsWindow`, which is where
//! they were first needed. The Import window draws the same cards and utility
//! buttons and cannot reach them there, so the two it shares live here as free
//! functions rather than being copied - two definitions of one card style
//! diverge the moment either is adjusted.
//!
//! Controls used by exactly one window stay with that window. This module is
//! for what genuinely crosses one.

use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::StyledExt;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::div;
use gpui_kit::rgb;

/// A titled, bordered card grouping related controls. The title is
/// deliberately larger than row content (`text_lg` vs. the default
/// `text_base` used by row labels/controls) - a section header should never
/// read smaller than what it's heading.
pub(crate) fn group(title: impl Into<gpui_kit::SharedString>) -> GroupBox {
    GroupBox::new().outline()
                   .title(div().text_lg().font_semibold().child(title.into()))
}

/// A small icon-only action button with a tooltip, used for utility actions
/// (choose/clear/add/edit/delete/copy/refresh) instead of a text label - text
/// buttons read as arbitrary activators, an icon reads as what it does.
/// `danger` tints destructive actions (clear/delete) red.
pub(crate) fn icon_button(id: impl Into<gpui_kit::ElementId>, icon_path: &'static str,
                          tooltip: impl Into<gpui_kit::SharedString>, danger: bool)
                          -> Button {
    let mut icon = Icon::default().path(icon_path);
    if danger {
        // `.ghost()` and `.danger()` are both button *variants* - only one can
        // apply, and ghost (no background) is what we want here - so tint the
        // icon itself red instead of switching variants.
        icon = icon.text_color(rgb(0xEF4444));
    }
    Button::new(id).icon(icon).tooltip(tooltip).ghost().small()
}
