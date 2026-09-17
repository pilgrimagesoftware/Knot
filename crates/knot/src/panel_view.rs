//! Renders `panel_state::PanelState` as a chat-like panel: streaming
//! messages, tool-call cards (by ACP `kind`, with a diff view for edit-kind
//! calls), and an inline permission prompt. Sibling to `terminal_view.rs`
//! (which renders a `Grid`) per design decision 5 - this renders a
//! completely different data model.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::{ClickEvent, IntoElement, ParentElement, Styled, div, rgb};
use knot_acp::{PermissionDecision, PermissionRequest};

use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};

const CARD_BG: u32 = 0x1E1E1E;
const CARD_BORDER: u32 = 0x333333;
const ERROR_COLOR: u32 = 0xEF4444;
const MUTED: u32 = 0x9CA3AF;

/// Renders the full panel: message list, then a pending permission prompt
/// or an ended-session banner if applicable. `on_permission_decision` is
/// invoked with the resolved decision when the user picks an option.
pub(crate) fn render_panel(state: &PanelState,
                           on_permission_decision: impl Fn(PermissionDecision) + Clone + 'static)
                           -> impl IntoElement {
    v_flex().size_full()
            .gap_3()
            .p_4()
            .overflow_y_hidden()
            .children(state.messages.iter().map(render_message))
            .children(state.pending_permission
                           .as_ref()
                           .map(|request| render_permission_prompt(request, on_permission_decision)))
            .children(state.ended.as_ref().map(render_ended_banner))
}

fn render_message(message: &PanelMessage) -> gpui_kit::AnyElement {
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
        PanelMessage::Assistant(text) => div().text_sm().child(text.clone()).into_any_element(),
        PanelMessage::ToolCall(card) => render_tool_call_card(card).into_any_element(),
    }
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
        let known =
            ["read", "edit", "delete", "move", "search", "execute", "think", "fetch"];
        for kind in known {
            assert_ne!(tool_call_icon(kind), IconName::Wrench,
                       "expected a specific icon for known kind {kind:?}");
        }
        assert_eq!(tool_call_icon("some-future-kind"), IconName::Wrench);
        assert_eq!(tool_call_icon(""), IconName::Wrench);
    }
}
