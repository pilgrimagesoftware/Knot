//! Unit tests for [`super`].

mod pull_requests;
mod shell;

use knot_acp::ConfigOption;
use knot_acp::PermissionRequest;
use knot_acp::SessionEndCause;
use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;
use knot_acp::ToolCallContent;
use serde_json::json;

use super::*;

fn text(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TextDelta { text: text.to_string(), })
}

#[test]
fn successive_text_deltas_accumulate_into_one_message() {
    let mut state = PanelState::new();

    state.apply(text("Hel"));
    state.apply(text("lo, "));
    state.apply(text("world"));

    assert_eq!(state.messages,
               vec![PanelMessage::Assistant("Hello, world".to_string())]);
}

#[test]
fn user_message_is_recorded_and_does_not_merge_with_assistant_text() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    state.apply(text("hi there"));

    assert_eq!(state.messages,
               vec![PanelMessage::User("hello".to_string()),
                    PanelMessage::Assistant("hi there".to_string())]);
}

/// An agent that answers a prompt with an error (an exhausted quota,
/// a dead transport) sends no `TurnEnd`, so the failure has to both
/// show in the conversation and release the composer.
#[test]
fn a_failed_prompt_is_recorded_as_an_error_and_ends_the_turn() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    state.push_error("The agent could not answer: quota exhausted".to_string());

    assert_eq!(
               state.messages,
               vec![
        PanelMessage::User("hello".to_string()),
        PanelMessage::Error("The agent could not answer: quota exhausted".to_string())
    ]
    );
    assert!(!state.turn_active,
            "a failed turn must not keep the composer blocked");
}

fn tool_call_start(id: &str, kind: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         kind.to_string(),
                                                        title:        String::new(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(), })
}

fn tool_call_update(id: &str, status: Option<&str>, content: Vec<ToolCallContent>) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallUpdate { tool_call_id: id.to_string(),
                                                         status: status.map(str::to_string),
                                                         title: None,
                                                         content })
}

#[test]
fn tool_call_lifecycle_builds_one_card() {
    let mut state = PanelState::new();

    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));
    state.apply(tool_call_update("tc1",
                                 Some("completed"),
                                 vec![ToolCallContent::Text("done".to_string())]));

    assert_eq!(
               state.messages,
               vec![PanelMessage::ToolCall(
        ToolCallCard { id:      "tc1".to_string(),
                       kind:    "execute".to_string(),
                       title:   String::new(),
                       status:  "completed".to_string(),
                       content: vec![ToolCallContent::Text("done".to_string())], }
    )]
    );
}

/// A finished call with no content must not keep reading as running -
/// the card renderer keys its in-progress placeholder on this, and
/// keying it on "no output yet" instead left completed cards stuck
/// showing "Running…".
#[test]
fn a_completed_tool_call_is_finished_even_with_no_content() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert!(card.is_finished());
    assert!(!card.failed());
    assert!(card.content.is_empty());
}

/// Per the ACP spec, every field but `toolCallId` is optional in an
/// update: one carrying only content must not blank out the status.
#[test]
fn a_content_only_update_leaves_the_status_alone() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
    state.apply(tool_call_update("tc1",
                                 None,
                                 vec![ToolCallContent::Text("late output".to_string())]));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.status, "completed");
    assert_eq!(card.content,
               vec![ToolCallContent::Text("late output".to_string())]);
}

#[test]
fn edit_tool_call_attaches_a_diff() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "edit"));

    state.apply(tool_call_update("tc1",
                                 Some("completed"),
                                 vec![ToolCallContent::Diff { path:     "src/lib.rs".to_string(),
                                                              old_text: Some("old".to_string()),
                                                              new_text: "new".to_string(), }]));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.content,
               vec![ToolCallContent::Diff { path:     "src/lib.rs".to_string(),
                                            old_text: Some("old".to_string()),
                                            new_text: "new".to_string(), }]);
}

/// The legacy `diff` session update carries no `toolCallId`, so it
/// attaches to the most recent card - kept working because
/// `acp-client`'s streaming requirement names diffs as a distinguished
/// event, even though no shipping agent emits one this way.
#[test]
fn a_standalone_diff_update_attaches_to_the_last_card() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "edit"));

    state.apply(SessionEvent::Update(SessionUpdate::Diff { path: "src/lib.rs".to_string(),
                                                           diff: "-old\n+new".to_string(), }));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.content,
               vec![ToolCallContent::Diff { path:     "src/lib.rs".to_string(),
                                            old_text: None,
                                            new_text: "-old\n+new".to_string(), }]);
}

#[test]
fn turn_end_mid_tool_call_keeps_the_cards_last_known_state() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason:
                                                                  "end_turn".to_string(), }));

    let PanelMessage::ToolCall(card) = &state.messages[0]
    else {
        panic!("expected a tool call card");
    };
    assert_eq!(card.status, "in_progress");
    assert!(!card.is_finished());
    assert!(card.content.is_empty());
}

#[test]
fn permission_request_is_pending_until_resolved() {
    let mut state = PanelState::new();
    let request = PermissionRequest { rpc_id:          json!(1),
                                      tool_call_id:    "tc1".to_string(),
                                      tool_call_title: None,
                                      options:         Vec::new(), };

    state.apply(SessionEvent::PermissionRequest(request.clone()));
    assert_eq!(state.pending_permission, Some(request));

    state.resolve_permission();
    assert_eq!(state.pending_permission, None);
}

#[test]
fn session_end_is_recorded() {
    let mut state = PanelState::new();

    state.apply(SessionEvent::Ended(SessionEndCause::ProcessExited { code: Some(1) }));

    assert_eq!(state.ended,
               Some(SessionEndCause::ProcessExited { code: Some(1) }));
}

#[test]
fn user_message_starts_a_tracked_turn_and_turn_end_ends_it() {
    let mut state = PanelState::new();

    state.push_user_message("hello".to_string());
    assert!(state.turn_active);
    assert!(state.tracking);

    state.apply(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason:
                                                                  "end_turn".to_string(), }));
    assert!(!state.turn_active);
}

#[test]
fn scrolling_away_clears_tracking_and_a_new_turn_resets_it() {
    let mut state = PanelState::new();
    state.push_user_message("hello".to_string());

    state.clear_tracking();
    assert!(!state.tracking);

    state.push_user_message("again".to_string());
    assert!(state.tracking);
}

#[test]
fn toggle_tracking_flips_the_flag() {
    let mut state = PanelState::new();
    state.push_user_message("hello".to_string());

    state.toggle_tracking();
    assert!(!state.tracking);
    state.toggle_tracking();
    assert!(state.tracking);
}

/// The card at `index`, for the collapse tests below.
fn card(state: &PanelState, index: usize) -> &ToolCallCard {
    let PanelMessage::ToolCall(card) = &state.messages[index]
    else {
        panic!("expected a tool call card");
    };
    card
}

#[test]
fn an_untouched_running_call_is_expanded() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    assert!(!state.is_collapsed(card(&state, 0)),
            "a pending call must stay open");

    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));
    assert!(!state.is_collapsed(card(&state, 0)),
            "a running call's output is what the user is waiting on");
}

#[test]
fn an_untouched_completed_call_is_collapsed() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    assert!(state.is_collapsed(card(&state, 0)));
}

/// The one place this reads past "collapse when they're done": a
/// failure is done, and it is also the card the user opened the
/// conversation to read.
#[test]
fn an_untouched_failed_call_stays_expanded() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("failed"), Vec::new()));

    assert!(!state.is_collapsed(card(&state, 0)));
}

#[test]
fn a_call_opened_while_running_stays_open_when_it_completes() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    // Running calls render open, so a toggle closes one; toggling
    // again is the user explicitly opening it.
    state.toggle_tool_call("tc1");
    state.toggle_tool_call("tc1");
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    assert!(!state.is_collapsed(card(&state, 0)),
            "a call the user opened must not slam shut on completion");
}

#[test]
fn a_call_closed_while_running_stays_closed_when_it_completes() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    state.toggle_tool_call("tc1");
    assert!(state.is_collapsed(card(&state, 0)),
            "a running call can be closed and goes on streaming out of sight");

    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
    assert!(state.is_collapsed(card(&state, 0)));
}

#[test]
fn a_call_closed_while_running_stays_closed_when_it_fails() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

    state.toggle_tool_call("tc1");
    state.apply(tool_call_update("tc1", Some("failed"), Vec::new()));

    assert!(state.is_collapsed(card(&state, 0)),
            "the user's choice outranks the automatic expansion of a failure");
}

#[test]
fn opening_one_call_leaves_the_others_alone() {
    let mut state = PanelState::new();
    for id in ["tc1", "tc2"] {
        state.apply(tool_call_start(id, "read"));
        state.apply(tool_call_update(id, Some("completed"), Vec::new()));
    }

    state.toggle_tool_call("tc1");

    assert!(!state.is_collapsed(card(&state, 0)));
    assert!(state.is_collapsed(card(&state, 1)));
}

#[test]
fn toggling_an_unknown_call_records_nothing() {
    let mut state = PanelState::new();

    state.toggle_tool_call("no-such-call");

    assert!(state.tool_call_collapsed.is_empty());
}

/// Nothing about the open/closed choices is persisted: the overrides
/// live in the panel's view state, so a reloaded conversation starts
/// from the automatic behaviour again.
#[test]
fn a_reloaded_conversation_collapses_a_call_the_previous_one_had_open() {
    let mut state = PanelState::new();
    state.apply(tool_call_start("tc1", "execute"));
    state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
    state.toggle_tool_call("tc1");
    assert!(!state.is_collapsed(card(&state, 0)));

    let mut reloaded = PanelState::new();
    reloaded.apply(tool_call_start("tc1", "execute"));
    reloaded.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

    assert!(reloaded.is_collapsed(card(&reloaded, 0)));
}

#[test]
fn config_option_update_replaces_the_declared_options() {
    let mut state = PanelState::new();
    let option = ConfigOption { id:            "mode".to_string(),
                                name:          "Mode".to_string(),
                                category:      Some("mode".to_string()),
                                kind:          "select".to_string(),
                                current_value: json!("code"),
                                options:       Vec::new(), };

    state.apply(SessionEvent::Update(SessionUpdate::ConfigOptionUpdate { config_options:
                                                                        vec![option.clone()], }));

    assert_eq!(state.config_options, vec![option]);
}

#[test]
fn usage_update_stores_valid_context_window_size() {
    let mut state = PanelState::new();

    state.apply(SessionEvent::Update(SessionUpdate::Usage { used: 53_000,
                                                            size: 200_000, }));

    assert_eq!(state.context_usage, Some((53_000, 200_000)));
}

fn permission(tool_call_id: &str, title: Option<&str>) -> PermissionRequest {
    PermissionRequest { rpc_id:          json!(1),
                        tool_call_id:    tool_call_id.to_string(),
                        tool_call_title: title.map(str::to_string),
                        options:         Vec::new(), }
}

#[test]
fn display_name_prefers_request_title_then_card_title_kind_then_id() {
    let mut state = PanelState::new();
    state.messages
         .push(PanelMessage::ToolCall(ToolCallCard { id:      "tc1".to_string(),
                                                     kind:    "read".to_string(),
                                                     title:
                                                         "Reading configuration file".to_string(),
                                                     status:  "pending".to_string(),
                                                     content: Vec::new(), }));

    assert_eq!(state.display_name(&permission("tc1", Some("Wire title"))),
               "Wire title");
    assert_eq!(state.display_name(&permission("tc1", None)),
               "Reading configuration file");

    state.messages
         .push(PanelMessage::ToolCall(ToolCallCard { id:      "tc2".to_string(),
                                                     kind:    "execute".to_string(),
                                                     title:   String::new(),
                                                     status:  "pending".to_string(),
                                                     content: Vec::new(), }));
    assert_eq!(state.display_name(&permission("tc2", None)), "execute");
    assert_eq!(state.display_name(&permission("missing", None)), "missing");
}
