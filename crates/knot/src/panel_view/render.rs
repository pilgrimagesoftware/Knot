//! The panel's element tree: the virtualized list, one row of it, the inline
//! permission prompt and the ended-session banner.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::{ClickEvent, IntoElement, ListState, ParentElement, Styled, div, rgb};
use knot_acp::PermissionDecision;
use knot_acp::PermissionRequest;
use parking_lot::Mutex;

use super::callbacks::PanelCallbacks;
use super::message::*;
use super::rows::*;
use super::style::ERROR_COLOR;
use super::style::PanelStyle;
use super::style::RiskLevel;
use super::style::risk_color;
use super::summary_row::*;
use crate::panel_state::PanelState;

/// How much extra space above and below the viewport the list lays out and
/// measures, so scrolling does not pop rows in at the edges.
pub(crate) const LIST_OVERDRAW: f32 = 400.;

/// Renders the full panel as one virtualized list: every message, then a
/// pending permission prompt or an ended-session banner if applicable.
/// Only rows intersecting the viewport (and a modest overdraw) are laid
/// out and measured, so per-frame cost tracks the pane's size rather than
/// the length of the conversation.
///
/// `list` is the caller's `ListState` and must already hold `row_count`
/// items (see `sync_row_count`); the returned element is the `list`
/// scroller itself, which fills the pane. The callbacks in `callbacks` drive
/// the panel's controls: the permission decision, the auto-scroll toggle for
/// the in-flight response, opening or closing a tool-call card by its id, and
/// a manual jump from a message's action bar - the last so the caller can
/// drop auto-scroll without the reconciler pulling the view back to the tail.
pub(crate) fn render_panel(state: Arc<Mutex<PanelState>>, list: ListState, style: &PanelStyle,
                           callbacks: PanelCallbacks)
                           -> impl IntoElement {
    let row_state = Arc::clone(&state);
    let row_list = list.clone();
    let row_style = style.clone();
    gpui_kit::list(list.clone(), move |index, _window, _cx| {
        let state = row_state.lock();
        render_row(index, &state, &row_style, &row_list, &callbacks)
    }).size_full()
      // `min_w_0` so a wide child (a markdown table, a long command line)
      // clips instead of stretching the pane and pushing the input row's
      // Send button off screen, per `knot-ui-conventions.md`'s "Flex
      // overflow" rule.
      .min_w_0()
      // Only vertical padding survives on the list itself: the virtualizer
      // lays each row out at the full viewport width and paints it at x=0,
      // so `px_*` here would be ignored and the sides would collapse. The
      // side inset and the gap between rows live on each row instead (see
      // `render_row`); top and bottom padding is honoured here.
      .pt_2()
      .pb_2()
      // Set the body size once, here, and let it cascade: a size applied to
      // the `TextView` itself reaches its paint but not the line wrapper's
      // measuring pass, so runs got measured at one size and drawn at
      // another and overlapped each other - worst around inline code, which
      // is measured separately in the mono family.
      .text_size(style.markdown_font_size)
}

/// Renders a single list row by resolving it against the current panel
/// state. An index past the end draws nothing rather than panicking, so a
/// frame racing a `sync_row_count` splice cannot crash the window.
///
/// Each row carries its own side inset and vertical margin: the virtualizer
/// ignores the list's horizontal padding and has no gap concept, so this is
/// where the content gets its breathing room from the pane edges and from
/// neighbouring rows.
fn render_row(index: usize, state: &PanelState, style: &PanelStyle, list: &ListState,
              callbacks: &PanelCallbacks)
              -> gpui_kit::AnyElement {
    let row = match row_at(state, index) {
        Some(PanelRow::Message(message_index)) => {
            match compact_row(message_index, state, style.compact_tool_calls) {
                // A tool call the run ahead of it already summarizes: it
                // draws nothing at all, not an empty padded row, so a run
                // reads as the single line it is meant to be.
                CompactRow::Covered => return div().into_any_element(),
                CompactRow::Summary => {
                    let head = state.tool_run_head(message_index)
                                    .unwrap_or_default()
                                    .to_owned();
                    render_tool_run_summary(state.tool_run_summary(message_index),
                                            head,
                                            style,
                                            callbacks.on_toggle_tool_run.clone()).into_any_element()
                }
                CompactRow::Message => {
                    let last_index = state.messages.len().checked_sub(1);
                    let message = &state.messages[message_index];
                    render_message(Message { state,
                                             index: message_index,
                                             is_last: Some(message_index) == last_index,
                                             style,
                                             list },
                                   message,
                                   callbacks)
                }
            }
        }
        Some(PanelRow::Permission) => {
            let request = state.pending_permission
                               .as_ref()
                               .expect("row_at yields Permission only while a request is pending");
            render_permission_prompt(state,
                                     request,
                                     style.permission_risk,
                                     callbacks.on_permission_decision.clone()).into_any_element()
        }
        Some(PanelRow::Ended) => {
            let cause = state.ended
                             .as_ref()
                             .expect("row_at yields Ended only once the session has ended");
            render_ended_banner(cause).into_any_element()
        }
        Some(PanelRow::Working) => crate::app_support::working_knot_animation().into_any_element(),
        None => return div().into_any_element(),
    };
    div().w_full()
         .min_w_0()
         .px_4()
         .py_2()
         .child(row)
         .into_any_element()
}

/// An inline permission request with actionable allow/deny controls, per
/// `acp-panel-ui`'s permission-prompts requirement. Sending further
/// prompts is blocked by the caller while this is rendered (the caller
/// checks `PanelState::pending_permission` before calling `prompt`).
fn render_permission_prompt(panel_state: &PanelState, request: &PermissionRequest,
                            permission_risk: RiskLevel,
                            on_decision: Rc<dyn Fn(PermissionDecision)>)
                            -> impl IntoElement {
    let allow = on_decision.clone();
    let deny = on_decision;
    v_flex()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(risk_color(permission_risk).unwrap_or(0x3B82F6)))
        .child(div().text_sm().child(format!(
            "Permission requested for {}",
            panel_state.display_name(request)
        )))
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("panel-permission-allow")
                        .label(knot_core::l10n::t("panel.allow"))
                        .primary()
                        .small()
                        .on_click(move |_: &ClickEvent, _, _| {
                            allow(PermissionDecision::Allow);
                        }),
                )
                .child(
                    Button::new("panel-permission-deny")
                        .label(knot_core::l10n::t("panel.deny"))
                        .ghost()
                        .small()
                        .on_click(move |_: &ClickEvent, _, _| {
                            deny(PermissionDecision::Deny);
                        }),
                ),
        )
}

fn render_ended_banner(cause: &knot_acp::SessionEndCause) -> impl IntoElement {
    div().text_xs()
         .text_color(rgb(ERROR_COLOR))
         .child(format!("Session ended: {cause}"))
}
