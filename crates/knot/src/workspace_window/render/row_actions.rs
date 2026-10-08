//! The actions button on an issue or change row (#504): a "…" button opening
//! the same menu as the row's context menu.
//!
//! The row itself is clickable - an issue row opens the issue - and gpui
//! fires click handlers in the bubble phase, so a click inside the row goes
//! on to the row's handler (`tests::pull_request_row_clicks` pins that
//! dispatch). Today the button's dropdown happens to open its overlay on
//! mouse-down, so the click never completes on the row anyway; the wrapper
//! stops the click regardless, so "the actions button does not open the
//! issue" (`workspace-issues`) does not rest on the popover's timing. The
//! test checks the outcome with the real button, not either mechanism.

use gpui_kit::component::menu::{DropdownMenu, PopupMenu};
use gpui_kit::{
    App, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div,
};

/// The row's actions button, keyed by `id`, whose menu `build` fills.
pub(in crate::workspace_window) fn row_actions_button(id: SharedString,
                                                      build: impl Fn(PopupMenu,
                                                         &mut Window,
                                                         &mut Context<PopupMenu>)
                                                         -> PopupMenu
                                                      + 'static)
                                                      -> impl IntoElement {
    let button = crate::controls::icon_button(SharedString::from(format!("{id}-button")),
                                              "icons/ellipsis.svg",
                                              knot_core::l10n::t("changes_view.row_actions"),
                                              false).dropdown_menu(move |menu, window, cx| {
                                                        build(menu, window, cx)
                                                    });
    div().id(id)
         .flex_shrink_0()
         .on_click(|_, _: &mut Window, cx: &mut App| cx.stop_propagation())
         .child(button)
}

#[cfg(test)]
mod tests;
