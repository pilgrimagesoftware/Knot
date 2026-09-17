//! GPUI renderer for the folded ACP session state.

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::{IntoElement, ParentElement, Styled, div, px, rgb};
use knot_acp::PermissionDecision;
use serde_json::Value;

use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};

pub(crate) fn render(
    state: &PanelState, on_permission: impl Fn(PermissionDecision) + Clone + 'static,
) -> impl IntoElement {
    let mut messages = v_flex().size_full().gap_3().p_5();
    for message in &state.messages {
        messages = messages.child(match message {
            PanelMessage::Text(text) => message_bubble("Assistant", text).into_any_element(),
            PanelMessage::ToolCall(card) => tool_call_card(card).into_any_element(),
        });
    }

    if let Some(request) = &state.pending_permission {
        let allow = on_permission.clone();
        let deny = on_permission.clone();
        let mut prompt = v_flex()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(rgb(0xd6a84f))
            .child(div().font_semibold().child("Permission required"))
            .child(
                div()
                    .text_sm()
                    .child(format!("Tool call {}", request.tool_call_id)),
            );
        for option in &request.options {
            prompt = prompt.child(div().text_sm().child(option.name.clone()));
        }
        prompt = prompt.child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("panel-permission-allow")
                        .label("Allow")
                        .on_click(move |_, _, _| allow(PermissionDecision::Allow)),
                )
                .child(
                    Button::new("panel-permission-deny")
                        .label("Deny")
                        .ghost()
                        .on_click(move |_, _, _| deny(PermissionDecision::Deny)),
                ),
        );
        messages = messages.child(prompt);
    }

    if let Some(ended) = &state.ended {
        messages = messages.child(
            div()
                .text_sm()
                .text_color(rgb(0x888888))
                .child(format!("Session ended: {ended:?}")),
        );
    }

    div().size_full().child(messages)
}

fn message_bubble(label: &'static str, text: &str) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(div().text_xs().text_color(rgb(0x888888)).child(label))
        .child(
            div()
                .max_w(px(900.))
                .p_3()
                .rounded_lg()
                .bg(rgb(0x252525))
                .child(text.to_owned()),
        )
}

fn tool_call_card(card: &ToolCallCard) -> impl IntoElement {
    let kind = display_kind(&card.kind);
    let state = card.status.as_deref().unwrap_or(if card.result.is_some() {
        "completed"
    } else {
        "in progress"
    });
    let mut body = v_flex()
        .gap_2()
        .p_3()
        .rounded_lg()
        .border_1()
        .border_color(rgb(0x444444))
        .child(
            h_flex()
                .justify_between()
                .child(div().font_semibold().child(kind))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x888888))
                        .child(state.to_owned()),
                ),
        )
        .child(div().text_sm().child(card.id.to_string()));

    if let Some((path, diff)) = &card.diff {
        body = body.child(diff_view(path, diff));
    }
    if let Some(result) = &card.result {
        body = body.child(div().text_sm().child(value_summary(result)));
    }
    body
}

fn diff_view(path: &str, diff: &str) -> impl IntoElement {
    let mut lines = v_flex()
        .gap_0p5()
        .p_2()
        .rounded_lg()
        .bg(rgb(0x171717))
        .child(div().font_semibold().child(path.to_owned()));
    for line in diff.lines() {
        let color = if line.starts_with('+') {
            rgb(0x55b56a)
        } else if line.starts_with('-') {
            rgb(0xe06c75)
        } else {
            rgb(0x888888)
        };
        lines = lines.child(div().text_sm().text_color(color).child(line.to_owned()));
    }
    lines
}

fn display_kind(kind: &str) -> String {
    match kind {
        "execute" => "Command".to_owned(),
        "read" => "Read file".to_owned(),
        "edit" => "Edit file".to_owned(),
        "search" => "Search".to_owned(),
        "" => "Tool call".to_owned(),
        other => format!("Tool call ({other})"),
    }
}

fn value_summary(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}
