//! Row chrome shared by every kind of row the section lists: the column
//! widths, the body's scroll ceiling, and one trailing icon control.
//!
//! Separate from `process_row` because the section lists more than processes,
//! and a control defined inside one row kind is a control the next one copies.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::Icon;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    ClickEvent, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, div,
};

/// Width of the runtime column, wide enough for `04d 05h` at the section's
/// text size.
pub(super) const RUNTIME_WIDTH: f32 = 64.;
/// Width of the PID column, wide enough for a seven-digit identifier.
pub(super) const PID_WIDTH: f32 = 64.;
/// Width of the state column on a subagent row, wide enough for the longest
/// state word at the section's text size.
pub(super) const STATE_WIDTH: f32 = 64.;
/// How tall the expanded list grows before it scrolls, so a busy agent
/// cannot push the session pane off the window.
pub(super) const BODY_MAX_HEIGHT: f32 = 220.;

/// One trailing control: an icon with a tooltip, ghost, as the UI
/// conventions ask for an action with an obvious icon.
///
/// `disabled` drops the click handler and the tooltip rather than dimming a
/// live control - a terminate already in flight has nothing more to ask for.
pub(super) fn row_action(id: (&'static str, u64), icon: IconName, tooltip_key: &'static str,
                         tint: gpui_kit::Hsla, disabled: bool,
                         on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui_kit::App) + 'static)
                         -> gpui_kit::AnyElement {
    let tooltip = knot_core::l10n::t(tooltip_key);

    div().id(id)
         .p_1()
         .rounded_sm()
         .when(!disabled, |element| {
             element.cursor_pointer()
                    .on_click(on_click)
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
         })
         .child(Icon::new(icon).size_3().text_color(tint))
         .into_any_element()
}

/// One group's heading inside the expanded body.
///
/// The spec asks for two *labelled* groups rather than two runs of rows, so
/// that a subagent row cannot be read as a process row. Labels, not
/// indentation: nothing here is nested under anything, and indenting the
/// processes under the subagents would claim an attribution `ps` cannot
/// establish.
pub(super) fn group_label(key: &'static str, theme_color: gpui_kit::Hsla) -> gpui_kit::AnyElement {
    div().px_5()
         .pt_2()
         .pb_1()
         .text_xs()
         .font_semibold()
         .text_color(theme_color)
         .child(knot_core::l10n::t(key))
         .into_any_element()
}
