//! Renders `panel_state::PanelState` as a chat-like panel: streaming
//! messages, tool-call cards (icon by ACP `kind`, body from the call's
//! reported content, with a diff view for diff blocks), and an inline
//! permission prompt. Sibling to `terminal_view.rs` (which renders a
//! `Grid`) per design decision 5 - this renders a completely different
//! data model.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{
    ClickEvent, ClipboardItem, IntoElement, ParentElement, ScrollHandle, Styled, div, relative, rgb,
};
use knot_acp::{PermissionDecision, PermissionRequest};

use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};

const CARD_BG: u32 = 0x1E1E1E;
const CARD_BORDER: u32 = 0x333333;
const ERROR_COLOR: u32 = 0xEF4444;
const SAFE_COLOR: u32 = 0x22C55E;
const MUTED: u32 = 0x9CA3AF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RiskLevel {
    Danger,
    Safe,
    Neutral,
}

pub(crate) fn permission_risk_level(value: &str, name: &str) -> RiskLevel {
    let value = format!("{value} {name}").to_ascii_lowercase();
    if ["bypass", "yolo", "danger"].iter()
                                   .any(|word| value.contains(word))
    {
        RiskLevel::Danger
    }
    else if ["plan", "read"].iter().any(|word| value.contains(word)) {
        RiskLevel::Safe
    }
    else {
        RiskLevel::Neutral
    }
}

pub(crate) fn risk_color(risk: RiskLevel) -> Option<u32> {
    match risk {
        RiskLevel::Danger => Some(ERROR_COLOR),
        RiskLevel::Safe => Some(SAFE_COLOR),
        RiskLevel::Neutral => None,
    }
}

/// Renders the full panel: message list, then a pending permission prompt
/// or an ended-session banner if applicable. `on_permission_decision` is
/// invoked with the resolved decision when the user picks an option.
/// `scroll` backs the conversation's scroll container (the caller applies
/// `.track_scroll(&scroll)` to it) so per-message action buttons can jump
/// to a specific message. `on_toggle_track` flips auto-scroll for the
/// in-flight response.
pub(crate) fn render_panel(state: &PanelState, scroll: &ScrollHandle,
                           permission_risk: RiskLevel,
                           on_permission_decision: impl Fn(PermissionDecision) + Clone + 'static,
                           on_toggle_track: impl Fn() + Clone + 'static)
                           -> impl IntoElement {
    let last_index = state.messages.len().checked_sub(1);
    // `w_full`, never `size_full`: this is the *content* of the caller's
    // `overflow_y_scroll` container, so a full height would pin it to the
    // viewport and clip everything past one screenful instead of letting
    // the container scroll. `min_w_0` for the same reason horizontally -
    // without it a wide child (a markdown table, a long command line)
    // stretches the pane and pushes the input row's Send button off
    // screen, per `knot-ui-conventions.md`'s "Flex overflow" rule.
    v_flex()
        .w_full()
        .min_w_0()
        .gap_3()
        .p_4()
        .children(state.messages.iter().enumerate().map(|(index, message)| {
            render_message(
                state,
                index,
                Some(index) == last_index,
                message,
                scroll,
                on_toggle_track.clone(),
            )
        }))
        .children(state.pending_permission.as_ref().map(|request| {
            render_permission_prompt(request, permission_risk, on_permission_decision)
        }))
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
        PanelMessage::User(text) => h_flex().w_full()
                                            .min_w_0()
                                            .justify_end()
                                            .child(div().max_w(relative(0.85))
                                                        .min_w_0()
                                                        .text_sm()
                                                        .text_color(rgb(0xFFFFFF))
                                                        .px_3()
                                                        .py_1p5()
                                                        .rounded_md()
                                                        .bg(rgb(0x2563EB))
                                                        .child(text.clone()))
                                            .into_any_element(),
        PanelMessage::Assistant(text) => {
            v_flex().w_full()
                    .min_w_0()
                    .gap_1()
                    // Plain `w_full().min_w_0()`, deliberately *not* a
                    // scroll container: a scroll parent hands its child an
                    // unconstrained width, so the markdown measured its
                    // runs against one width and painted them into
                    // another, drawing words on top of each other. Wide
                    // content clips here instead, which `min_w_0` at least
                    // keeps from stretching the pane.
                    .child(div().w_full()
                                .min_w_0()
                                .child(TextView::markdown(("panel-message-markdown",
                                                           index as u64),
                                                          text.clone()).text_sm()))
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
    h_flex()
        .gap_1()
        .child(
            Button::new("panel-copy-response")
                .icon(IconName::Copy)
                .tooltip("Copy response")
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                }),
        )
        .children(user_index.map(|user_index| {
            Button::new("panel-scroll-to-user")
                .icon(IconName::ArrowUp)
                .tooltip("Scroll to your message")
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, _, _| {
                    scroll_to_user.scroll_to_top_of_item(user_index);
                })
        }))
        .child(
            Button::new("panel-scroll-to-top")
                .icon(IconName::ChevronsUp)
                .tooltip("Scroll to top")
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, _, _| {
                    scroll_to_top.scroll_to_top_of_item(0);
                }),
        )
}

/// A tool-call card: an icon/title/status header over whatever content
/// the agent has reported so far - diff blocks as an added/removed line
/// view, everything else as monospace output. An unfinished call with no
/// content yet shows an in-progress placeholder; a *finished* one with no
/// content shows nothing rather than a stale "Running…" (the bug this
/// keys the placeholder on `status` to avoid).
fn render_tool_call_card(card: &ToolCallCard) -> impl IntoElement {
    let label = if card.title.is_empty() {
        card.kind.clone()
    }
    else {
        card.title.clone()
    };
    v_flex().w_full()
            .min_w_0()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(if card.failed() { ERROR_COLOR } else { CARD_BORDER }))
            .bg(rgb(CARD_BG))
            .child(h_flex().w_full()
                           .min_w_0()
                           .gap_2()
                           .items_center()
                           .child(Icon::new(tool_call_icon(&card.kind)).xsmall()
                                                                       .text_color(rgb(MUTED)))
                           .child(div().flex_1()
                                       .min_w_0()
                                       .text_xs()
                                       .text_color(rgb(MUTED))
                                       .child(label))
                           .child(div().flex_shrink_0()
                                       .text_xs()
                                       .text_color(rgb(if card.failed() {
                                                           ERROR_COLOR
                                                       }
                                                       else {
                                                           MUTED
                                                       }))
                                       .child(status_label(&card.status))))
            .children(card.content.iter().map(render_tool_call_content))
            .children((card.content.is_empty() && !card.is_finished())
                                                                     .then(in_progress_placeholder))
}

/// One tool-call content block. Diffs get the added/removed line view
/// `acp-panel-ui`'s tool-call-rendering requirement asks for; text and
/// terminal references fall back to monospace output.
fn render_tool_call_content(content: &knot_acp::ToolCallContent) -> gpui_kit::AnyElement {
    match content {
        knot_acp::ToolCallContent::Diff { path,
                                          old_text,
                                          new_text, } => {
            render_diff(path, old_text.as_deref(), new_text).into_any_element()
        }
        knot_acp::ToolCallContent::Text(text) => render_output_text(text).into_any_element(),
        knot_acp::ToolCallContent::Terminal { terminal_id } => {
            render_output_text(&format!("[terminal {terminal_id}]")).into_any_element()
        }
    }
}

/// Human-readable form of an ACP tool-call status. Unrecognized statuses
/// pass through unchanged rather than being swallowed - `status` is a
/// plain wire string, and a future value is more useful shown than hidden.
fn status_label(status: &str) -> String {
    match status {
        "pending" => "Pending".to_string(),
        "in_progress" => "Running…".to_string(),
        "completed" => "Done".to_string(),
        "failed" => "Failed".to_string(),
        other => other.to_string(),
    }
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
/// per `acp-panel-ui`'s tool-call-rendering requirement. `old_text` is
/// `None` for a newly created file, in which case every line is an
/// addition.
fn render_diff(path: &str, old_text: Option<&str>, new_text: &str) -> impl IntoElement {
    v_flex().w_full()
            .min_w_0()
            .gap_1()
            .child(div().w_full()
                        .min_w_0()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(path.to_string()))
            .child(v_flex().w_full()
                           .min_w_0()
                           .children(diff_lines(old_text, new_text).into_iter()
                                                                   .map(|(color, text)| {
                                                                       div().w_full()
                                                                            .min_w_0()
                                                                            .font_family("monospace")
                                                                            .text_xs()
                                                                            .text_color(rgb(color))
                                                                            .child(text)
                                                                   })))
}

/// The `(color, text)` line list for a diff. With an `old_text` this is a
/// whole-file replacement, so every old line reads as removed and every
/// new line as added; without one the `new_text` is already in unified
/// form (or is a brand-new file) and its own `+`/`-` prefixes decide.
fn diff_lines(old_text: Option<&str>, new_text: &str) -> Vec<(u32, String)> {
    match old_text {
        Some(old) => old.lines()
                        .map(|line| (ERROR_COLOR, format!("- {line}")))
                        .chain(new_text.lines()
                                       .map(|line| (SAFE_COLOR, format!("+ {line}"))))
                        .collect(),
        None => new_text.lines()
                        .map(|line| match line.strip_prefix('+') {
                            Some(added) => (SAFE_COLOR, format!("+ {added}")),
                            None => match line.strip_prefix('-') {
                                Some(removed) => (ERROR_COLOR, format!("- {removed}")),
                                None => (MUTED, line.to_string()),
                            },
                        })
                        .collect(),
    }
}

/// Monospace tool output. `w_full`/`min_w_0` so a long line wraps inside
/// the card instead of stretching the whole conversation pane, per
/// `knot-ui-conventions.md`'s "Flex overflow" rule.
fn render_output_text(text: &str) -> impl IntoElement {
    div().w_full()
         .min_w_0()
         .font_family("monospace")
         .text_xs()
         .text_color(rgb(MUTED))
         .child(text.to_string())
}

/// An inline permission request with actionable allow/deny controls, per
/// `acp-panel-ui`'s permission-prompts requirement. Sending further
/// prompts is blocked by the caller while this is rendered (the caller
/// checks `PanelState::pending_permission` before calling `prompt`).
fn render_permission_prompt(request: &PermissionRequest, permission_risk: RiskLevel,
                            on_decision: impl Fn(PermissionDecision) + Clone + 'static)
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
            "Permission requested for tool call {}",
            request.tool_call_id
        )))
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("panel-permission-allow")
                        .label("Allow")
                        .primary()
                        .small()
                        .on_click(move |_: &ClickEvent, _, _| {
                            allow(PermissionDecision::Allow);
                        }),
                )
                .child(
                    Button::new("panel-permission-deny")
                        .label("Deny")
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

    /// The bug this guards: keying the in-progress placeholder on "no
    /// output yet" left a completed call showing "Running…" forever,
    /// because a call can finish without ever reporting content.
    #[test]
    fn only_a_running_status_reads_as_running() {
        assert_eq!(status_label("in_progress"), "Running…");
        assert_eq!(status_label("pending"), "Pending");
        assert_eq!(status_label("completed"), "Done");
        assert_eq!(status_label("failed"), "Failed");
    }

    #[test]
    fn an_unrecognized_status_passes_through_rather_than_vanishing() {
        assert_eq!(status_label("some_future_status"), "some_future_status");
    }

    #[test]
    fn a_replacement_diff_reads_as_removals_then_additions() {
        let lines = diff_lines(Some("a\nb"), "a\nc");

        assert_eq!(lines,
                   vec![(ERROR_COLOR, "- a".to_string()),
                        (ERROR_COLOR, "- b".to_string()),
                        (SAFE_COLOR, "+ a".to_string()),
                        (SAFE_COLOR, "+ c".to_string())]);
    }

    #[test]
    fn a_new_file_diff_is_all_additions_with_no_prefixes_to_strip() {
        let lines = diff_lines(None, "fn main() {}");

        assert_eq!(lines, vec![(MUTED, "fn main() {}".to_string())]);
    }

    #[test]
    fn a_unified_diff_without_old_text_colors_by_prefix() {
        let lines = diff_lines(None, "-old\n+new\n context");

        assert_eq!(lines,
                   vec![(ERROR_COLOR, "- old".to_string()),
                        (SAFE_COLOR, "+ new".to_string()),
                        (MUTED, " context".to_string())]);
    }

    #[test]
    fn permission_modes_are_classified_case_insensitively() {
        assert_eq!(permission_risk_level("bypassPermissions", "Restricted"),
                   RiskLevel::Danger);
        assert_eq!(permission_risk_level("unknown", "PLAN"), RiskLevel::Safe);
        assert_eq!(permission_risk_level("default", "Normal"),
                   RiskLevel::Neutral);
    }
}
