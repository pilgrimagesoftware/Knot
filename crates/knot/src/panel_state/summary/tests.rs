//! Unit tests for [`super`] - the compact tool-call summary's grouping
//! and counts, driven through the real event stream rather than by
//! building `PanelState` by hand, so the boundaries under test are the
//! ones a live session produces.

use knot_acp::{SessionEvent, SessionUpdate, ToolCallContent};

use super::*;

fn start(id: &str, kind: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         kind.to_string(),
                                                        title:        String::new(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(), })
}

fn update(id: &str, status: &str, content: Vec<ToolCallContent>) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallUpdate { tool_call_id: id.to_string(),
                                                         status: Some(status.to_string()),
                                                         title: None,
                                                         content })
}

fn text(text: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::TextDelta { text: text.to_string(), })
}

fn diff(path: &str) -> ToolCallContent {
    ToolCallContent::Diff { path:     path.to_string(),
                            old_text: None,
                            new_text: "+ new".to_string(), }
}

/// "Aggregate tool activity": three consecutive calls are one run, and
/// its counts cover all three.
#[test]
fn three_consecutive_calls_form_one_run() {
    let mut state = PanelState::new();
    state.apply(start("a", "read"));
    state.apply(start("b", "read"));
    state.apply(start("c", "execute"));

    assert!(state.starts_tool_run(0));
    assert!(state.continues_tool_run(1));
    assert!(state.continues_tool_run(2));

    let summary = state.tool_run_summary(0);
    assert_eq!(summary.calls, 3);
    assert_eq!(summary.files_read, 2);
    assert_eq!(summary.commands_run, 1);
}

/// "Assistant text separates summaries": a non-tool message is a hard
/// boundary, and the tool call after it heads a new run.
#[test]
fn assistant_text_between_calls_starts_a_new_run() {
    let mut state = PanelState::new();
    state.apply(start("a", "read"));
    state.apply(text("thinking"));
    state.apply(start("b", "read"));

    assert!(state.starts_tool_run(0));
    assert!(!state.starts_tool_run(1), "text is not a run");
    assert!(state.starts_tool_run(2),
            "the call after text heads a new run");

    assert_eq!(state.tool_run_summary(0).calls, 1);
    assert_eq!(state.tool_run_summary(2).calls, 1);
}

/// A user prompt is a boundary too - the next turn's calls never fold
/// into the previous turn's summary.
#[test]
fn a_user_prompt_separates_runs() {
    let mut state = PanelState::new();
    state.apply(start("a", "execute"));
    state.push_user_message("again".to_string());
    state.apply(start("b", "execute"));

    assert_eq!(state.tool_run_summary(0).calls, 1);
    assert!(state.starts_tool_run(2));
}

/// An error entry is a boundary on the same footing as text: it is a
/// non-tool message, so it closes the run in front of it.
#[test]
fn an_error_entry_separates_runs() {
    let mut state = PanelState::new();
    state.apply(start("a", "read"));
    state.push_error("quota exhausted".to_string());
    state.apply(start("b", "read"));

    assert_eq!(state.tool_run_summary(0).calls, 1);
    assert!(state.starts_tool_run(2));
}

/// "Update after a file edit": counts are read from the cards live, so
/// they move as results arrive without the run being rebuilt.
#[test]
fn counts_update_as_results_arrive() {
    let mut state = PanelState::new();
    state.apply(start("a", "edit"));
    assert_eq!(state.tool_run_summary(0).files_edited, 1);
    assert!(state.tool_run_summary(0).running,
            "a pending call reads as live");

    state.apply(update("a", "completed", vec![diff("src/one.rs")]));
    state.apply(start("b", "edit"));
    state.apply(update("b", "completed", vec![diff("src/two.rs")]));

    let summary = state.tool_run_summary(0);
    assert_eq!(summary.calls, 2);
    assert_eq!(summary.files_edited, 2);
    assert!(!summary.running, "every call finished");
}

/// The same file edited twice counts once - the run reports files, not
/// diff blocks.
#[test]
fn a_file_edited_twice_counts_once() {
    let mut state = PanelState::new();
    state.apply(start("a", "edit"));
    state.apply(update("a", "completed", vec![diff("src/one.rs")]));
    state.apply(start("b", "edit"));
    state.apply(update("b", "completed", vec![diff("src/one.rs")]));

    assert_eq!(state.tool_run_summary(0).files_edited, 1);
    assert_eq!(state.tool_run_summary(0).calls, 2);
}

/// "Failed call remains represented": the failure shows in the summary
/// and the card itself is still in panel state to be inspected.
#[test]
fn a_failed_call_is_counted_and_its_card_is_kept() {
    let mut state = PanelState::new();
    state.apply(start("a", "execute"));
    state.apply(start("b", "execute"));
    state.apply(update("b",
                       "failed",
                       vec![ToolCallContent::Text("boom".to_string())]));

    let summary = state.tool_run_summary(0);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.calls, 2);

    let card = state.messages
                    .iter()
                    .find_map(|message| match message {
                        PanelMessage::ToolCall(card) if card.id == "b" => Some(card),
                        _ => None,
                    })
                    .expect("the failed call's record survives compact rendering");
    assert_eq!(card.status, "failed");
    assert_eq!(card.content,
               vec![ToolCallContent::Text("boom".to_string())]);
}

/// A kind the summary does not model still counts as a call, and adds no
/// secondary count it cannot support.
#[test]
fn an_unmodelled_kind_counts_only_as_a_call() {
    let mut state = PanelState::new();
    state.apply(start("a", "think"));

    let summary = state.tool_run_summary(0);
    assert_eq!(summary.calls, 1);
    assert_eq!(summary.files_edited, 0);
    assert_eq!(summary.files_read, 0);
    assert_eq!(summary.commands_run, 0);
}

/// Every call in a run resolves to the same head id, which is the key a
/// run's open/closed choice is stored under.
#[test]
fn every_call_in_a_run_shares_one_head() {
    let mut state = PanelState::new();
    state.apply(start("a", "read"));
    state.apply(start("b", "read"));
    state.apply(text("done"));
    state.apply(start("c", "read"));

    assert_eq!(state.tool_run_head(0), Some("a"));
    assert_eq!(state.tool_run_head(1), Some("a"));
    assert_eq!(state.tool_run_head(2), None, "text heads no run");
    assert_eq!(state.tool_run_head(3), Some("c"));
}

/// Opening a run is a stored override, and it survives the run growing
/// another call afterwards.
#[test]
fn opening_a_run_outlives_the_run_growing() {
    let mut state = PanelState::new();
    state.apply(start("a", "read"));
    assert!(!state.is_tool_run_expanded("a"));

    state.toggle_tool_run("a".to_string());
    assert!(state.is_tool_run_expanded("a"));

    state.apply(start("b", "read"));
    assert!(state.is_tool_run_expanded("a"),
            "the run stays open as it grows");
    assert_eq!(state.tool_run_head(1), Some("a"));

    state.toggle_tool_run("a".to_string());
    assert!(!state.is_tool_run_expanded("a"));
}

/// Asking about an index past the end is answered, not panicked on - the
/// render path can briefly hold a stale index between a message landing
/// and the next row-count splice.
#[test]
fn an_index_past_the_end_is_answered_safely() {
    let state = PanelState::new();
    assert!(!state.starts_tool_run(7));
    assert!(!state.continues_tool_run(7));
    assert_eq!(state.tool_run_head(7), None);
    assert_eq!(state.tool_run_summary(7), ToolRunSummary::default());
}
