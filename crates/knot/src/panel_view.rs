//! Renders `panel_state::PanelState` as a chat-like panel: streaming
//! messages, tool-call cards (by ACP `kind`, with a diff view for edit-kind
//! calls), and an inline permission prompt. Sibling to `terminal_view.rs`
//! (which renders a `Grid`) per design decision 5 - this renders a
//! completely different data model.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{
    ClickEvent, ClipboardItem, IntoElement, ParentElement, ScrollHandle, Styled, div, rgb,
};
use knot_acp::{PermissionDecision, PermissionRequest};

use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};

const CARD_BG: u32 = 0x1E1E1E;
const CARD_BORDER: u32 = 0x333333;
const ERROR_COLOR: u32 = 0xEF4444;
const MUTED: u32 = 0x9CA3AF;

/// Renders the full panel: message list, then a pending permission prompt
/// or an ended-session banner if applicable. `on_permission_decision` is
/// invoked with the resolved decision when the user picks an option.
/// `scroll` backs the conversation's scroll container (the caller applies
/// `.track_scroll(&scroll)` to it) so per-message action buttons can jump
/// to a specific message. `on_toggle_track` flips auto-scroll for the
/// in-flight response.
pub(crate) fn render_panel(state: &PanelState, scroll: &ScrollHandle,
                           on_permission_decision: impl Fn(PermissionDecision) + Clone + 'static,
                           on_toggle_track: impl Fn() + Clone + 'static)
                           -> impl IntoElement {
    let last_index = state.messages.len().checked_sub(1);
    v_flex().size_full()
            .gap_3()
            .p_4()
            .overflow_y_hidden()
            .children(state.messages.iter().enumerate().map(|(index, message)| {
                                                            render_message(state,
                                                                           index,
                                                                           Some(index) == last_index,
                                                                           message,
                                                                           scroll,
                                                                           on_toggle_track.clone())
                                                        }))
            .children(state.pending_permission
                           .as_ref()
                           .map(|request| render_permission_prompt(request, on_permission_decision)))
            .children(state.ended.as_ref().map(render_ended_banner))
}

fn render_message(state: &PanelState, index: usize, is_last: bool, message: &PanelMessage,
                  scroll: &ScrollHandle, on_toggle_track: impl Fn() + Clone + 'static)
                  -> gpui_kit::AnyElement {
    match message {
        // Right-aligned, tinted background - visually distinct from the
        // assistant's plain left-aligned text, per acp-panel-ui's
        // "visually distinguish user messages, assistant messages, and
        // system/tool content" requirement.
        PanelMessage::User(text) => h_flex().justify_end()
                                            .child(div().text_sm()
                                                        .text_color(rgb(0xFFFFFF))
                                                        .px_3()
                                                        .py_1p5()
                                                        .rounded_md()
                                                        .bg(rgb(0x2563EB))
                                                        .child(text.clone()))
                                            .into_any_element(),
        PanelMessage::Assistant(text) => {
            v_flex().gap_1()
                    .child(TextView::markdown(("panel-message-markdown", index as u64),
                                              text.clone()).text_sm())
                    .children((is_last && state.turn_active).then(|| {
                                                                render_track_toggle(state.tracking,
                                                                                    on_toggle_track)
                                                            }))
                    .children((!(is_last && state.turn_active)).then(|| {
                                  let user_index = preceding_user_message(state, index);
                                  render_response_actions(text.clone(), user_index, scroll)
                              }))
                    .into_any_element()
        }
        PanelMessage::ToolCall(card) => render_tool_call_card(card).into_any_element(),
    }
}

/// The index of the nearest `PanelMessage::User` before `index`, for the
/// response action bar's "scroll to user input" control.
fn preceding_user_message(state: &PanelState, index: usize) -> Option<usize> {
    state.messages[..index].iter()
                           .rposition(|message| matches!(message, PanelMessage::User(_)))
}

/// The in-flight response's auto-scroll toggle, per the track toggle
/// design's per-response scope - shown only on the currently streaming
/// response, replaced by the response action bar once it finalizes.
fn render_track_toggle(tracking: bool, on_toggle: impl Fn() + Clone + 'static) -> impl IntoElement {
    h_flex().child(Button::new("panel-track-toggle").icon(if tracking {
                                                              IconName::CircleDot
                                                          }
                                                          else {
                                                              IconName::Circle
                                                          })
                                                    .tooltip(if tracking {
                                                                 "Following new output"
                                                             }
                                                             else {
                                                                 "Not following new output"
                                                             })
                                                    .ghost()
                                                    .small()
                                                    .on_click(move |_: &ClickEvent, _, _| {
                                                        on_toggle()
                                                    }))
}

/// A finalized response's action bar: copy, scroll to the user message that
/// prompted it, and scroll to the top of the conversation.
fn render_response_actions(text: String, user_index: Option<usize>, scroll: &ScrollHandle)
                           -> impl IntoElement {
    let scroll_to_user = scroll.clone();
    let scroll_to_top = scroll.clone();
    h_flex().gap_1()
            .child(Button::new("panel-copy-response").icon(IconName::Copy)
                                                      .tooltip("Copy response")
                                                      .ghost()
                                                      .small()
                                                      .on_click(move |_: &ClickEvent, _, cx| {
                                                          cx.write_to_clipboard(
                                                              ClipboardItem::new_string(
                                                                  text.clone(),
                                                              ),
                                                          );
                                                      }))
            .children(user_index.map(|user_index| {
                Button::new("panel-scroll-to-user").icon(IconName::ArrowUp)
                                                    .tooltip("Scroll to your message")
                                                    .ghost()
                                                    .small()
                                                    .on_click(move |_: &ClickEvent, _, _| {
                                                        scroll_to_user.scroll_to_top_of_item(user_index);
                                                    })
            }))
            .child(Button::new("panel-scroll-to-top").icon(IconName::ChevronsUp)
                                                      .tooltip("Scroll to top")
                                                      .ghost()
                                                      .small()
                                                      .on_click(move |_: &ClickEvent, _, _| {
                                                          scroll_to_top.scroll_to_top_of_item(0);
                                                      }))
}

/// A tool-call card, rendered by ACP `kind`: edit-kind calls show an
/// added/removed diff view; every other kind (including unrecognized ones)
/// falls back to a generic status/output card.
fn render_tool_call_card(card: &ToolCallCard) -> impl IntoElement {
    v_flex().gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .bg(rgb(CARD_BG))
            .child(h_flex().gap_2()
                           .items_center()
                           .child(Icon::new(tool_call_icon(&card.kind)).xsmall()
                                                                       .text_color(rgb(MUTED)))
                           .child(div().text_xs()
                                       .text_color(rgb(MUTED))
                                       .child(card.kind.clone()))
                           .children(card.status.clone().map(|status| {
                                                            div().text_xs()
                                                                 .text_color(rgb(MUTED))
                                                                 .child(status)
                                                        })))
            .child(if card.kind == "edit" {
                       card.diff
                           .as_ref()
                           .map(|(path, diff)| render_diff(path, diff).into_any_element())
                           .unwrap_or_else(|| in_progress_placeholder().into_any_element())
                   }
                   else {
                       card.result
                           .as_ref()
                           .map(|output| render_generic_output(output).into_any_element())
                           .unwrap_or_else(|| in_progress_placeholder().into_any_element())
                   })
}

/// Maps an ACP tool-call `kind` to an identifying icon, per the response
/// action bar design's "icon lookup keyed on kind" decision. `kind` is a
/// plain string off the wire rather than a closed enum, so the match is
/// string-keyed with a generic fallback arm rather than truly exhaustive.
fn tool_call_icon(kind: &str) -> IconName {
    match kind {
        "read" => IconName::FileText,
        "edit" => IconName::Pencil,
        "delete" => IconName::Trash,
        "move" => IconName::Move,
        "search" => IconName::Search,
        "execute" => IconName::Terminal,
        "think" => IconName::Brain,
        "fetch" => IconName::Globe,
        _ => IconName::Wrench,
    }
}

fn in_progress_placeholder() -> impl IntoElement {
    div().text_xs().text_color(rgb(MUTED)).child("Running…")
}

/// A file-edit diff as an added/removed line view rather than raw text,
/// per `acp-panel-ui`'s tool-call-rendering requirement.
fn render_diff(path: &str, diff: &str) -> impl IntoElement {
    v_flex().gap_1()
            .child(div().text_xs()
                        .text_color(rgb(MUTED))
                        .child(path.to_string()))
            .children(diff.lines().map(|line| {
                                      let (color, text) =
                                          if let Some(added) = line.strip_prefix('+') {
                                              (0x22C55E, format!("+ {added}"))
                                          }
                                          else if let Some(removed) = line.strip_prefix('-') {
                                              (0xEF4444, format!("- {removed}"))
                                          }
                                          else {
                                              (MUTED, line.to_string())
                                          };
                                      div().font_family("monospace")
                                           .text_xs()
                                           .text_color(rgb(color))
                                           .child(text)
                                  }))
}

/// A generic input/output fallback for tool-call kinds without a dedicated
/// renderer (execute, read, and any unrecognized kind).
fn render_generic_output(output: &serde_json::Value) -> impl IntoElement {
    div().font_family("monospace")
         .text_xs()
         .text_color(rgb(MUTED))
         .child(serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string()))
}

/// An inline permission request with actionable allow/deny controls, per
/// `acp-panel-ui`'s permission-prompts requirement. Sending further
/// prompts is blocked by the caller while this is rendered (the caller
/// checks `PanelState::pending_permission` before calling `prompt`).
fn render_permission_prompt(request: &PermissionRequest,
                            on_decision: impl Fn(PermissionDecision) + Clone + 'static)
                            -> impl IntoElement {
    let allow = on_decision.clone();
    let deny = on_decision;
    v_flex().gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x3B82F6))
            .child(div().text_sm().child(format!("Permission requested for tool call {}",
                                                 request.tool_call_id)))
            .child(h_flex().gap_2()
                           .child(Button::new("panel-permission-allow").label("Allow")
                                                                       .primary()
                                                                       .small()
                                                                       .on_click(move |_: &ClickEvent, _, _| {
                                                                           allow(PermissionDecision::Allow);
                                                                       }))
                           .child(Button::new("panel-permission-deny").label("Deny")
                                                                      .ghost()
                                                                      .small()
                                                                      .on_click(move |_: &ClickEvent, _, _| {
                                                                          deny(PermissionDecision::Deny);
                                                                      })))
}

fn render_ended_banner(cause: &knot_acp::SessionEndCause) -> impl IntoElement {
    div().text_xs()
         .text_color(rgb(ERROR_COLOR))
         .child(format!("Session ended: {cause}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_kind_maps_to_a_distinct_icon_and_unknown_kinds_fall_back() {
        let known = ["read", "edit", "delete", "move", "search", "execute", "think", "fetch"];
        for kind in known {
            assert_ne!(tool_call_icon(kind),
                       IconName::Wrench,
                       "expected a specific icon for known kind {kind:?}");
        }
        assert_eq!(tool_call_icon("some-future-kind"), IconName::Wrench);
        assert_eq!(tool_call_icon(""), IconName::Wrench);
    }
}
