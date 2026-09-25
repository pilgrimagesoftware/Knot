//! Noticing pull request URLs in a panel agent's tool-call output.

use knot_acp::{SessionEvent, SessionUpdate, ToolCallContent};

use crate::panel_state::PanelState;

const PULL_REQUEST: &str = "https://github.com/acme/widget/pull/42";

fn start(id: &str, kind: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         kind.to_string(),
                                                        title:        String::new(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    None,
                                                        meta:         None, })
}

fn update(id: &str, content: Vec<ToolCallContent>) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallUpdate { tool_call_id: id.to_string(),
                                                         status: Some("completed".to_string()),
                                                         title: None,
                                                         content,
                                                         raw_input: None,
                                                         meta: None })
}

fn text(body: impl Into<String>) -> Vec<ToolCallContent> {
    vec![ToolCallContent::Text(body.into())]
}

#[test]
fn a_url_in_a_tool_calls_content_is_noted() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "execute"));

    state.apply(update("tc1", text(PULL_REQUEST)));

    assert_eq!(state.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

#[test]
fn a_url_in_a_tool_call_result_is_noted() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "execute"));

    state.apply(SessionEvent::Update(SessionUpdate::ToolCallResult {
        tool_call_id: "tc1".to_string(),
        output:       serde_json::json!({ "stdout": format!("Created {PULL_REQUEST}") }),
    }));

    assert_eq!(state.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

/// A tool-call update replaces a card's content wholesale, so the same text
/// folds in more than once. The buffer must not grow with every re-render.
#[test]
fn the_same_url_folded_in_twice_is_buffered_once() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "execute"));

    state.apply(update("tc1", text(PULL_REQUEST)));
    state.apply(update("tc1", text(PULL_REQUEST)));

    assert_eq!(state.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

/// A decorated URL is the same pull request, so what is handed over is the
/// canonical form rather than what the output happened to carry.
#[test]
fn a_decorated_url_is_buffered_in_its_canonical_form() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "execute"));

    state.apply(update("tc1", text(format!("{PULL_REQUEST}/files"))));

    assert_eq!(state.take_pull_request_urls(),
               vec![PULL_REQUEST.to_string()]);
}

#[test]
fn draining_clears_the_buffer() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "execute"));
    state.apply(update("tc1", text(PULL_REQUEST)));

    assert_eq!(state.take_pull_request_urls().len(), 1);
    assert!(state.take_pull_request_urls().is_empty());
}

/// A diff's body is a patch, not output an agent printed.
#[test]
fn a_diff_body_is_not_scanned() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "edit"));

    state.apply(update("tc1",
                       vec![ToolCallContent::Diff { path:     "README.md".to_string(),
                                                    old_text: None,
                                                    new_text: format!("+ see {PULL_REQUEST}"), }]));

    assert!(state.take_pull_request_urls().is_empty());
}

#[test]
fn output_with_no_pull_request_url_notes_nothing() {
    let mut state = PanelState::new();
    state.apply(start("tc1", "execute"));

    state.apply(update("tc1", text("https://github.com/acme/widget/issues/42")));

    assert!(state.take_pull_request_urls().is_empty());
}
