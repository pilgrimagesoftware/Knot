//! The Claude Code recognizer, driven entirely off committed fixtures.
//!
//! No test in this file writes ACP JSON inline. The fixtures are the contract
//! with `@agentclientprotocol/claude-agent-acp`, and a test that built its own
//! input would be asserting against this crate's idea of the wire shape rather
//! than against the adapter's - which is the mistake the fixtures exist to
//! prevent. See `tests/fixtures/claude/README.md`.
//!
//! The one exception is a test that *removes* a field from a fixture to prove
//! a degradation, which is still anchored to the real shape.

use knot_subagents::recognize::{Recognizer, ToolCallReport, for_agent_type};
use knot_subagents::{Outcome, SubagentEvent, SubagentId, SubagentKind};
use serde_json::Value;

const DISPATCH: &str = include_str!("fixtures/claude/dispatch.json");
const NOT_A_DELEGATION: &str = include_str!("fixtures/claude/not_a_delegation.json");
const COMPLETION: &str = include_str!("fixtures/claude/completion.json");
const COMPLETION_FAILED: &str = include_str!("fixtures/claude/completion_failed.json");

/// The `update` object out of a fixture's `session/update` envelope - the same
/// unwrapping `SessionUpdate::from_params` does.
fn update(fixture: &str) -> Value {
    let params: Value = serde_json::from_str(fixture).expect("fixture is valid JSON");

    params.get("update").expect("fixture has an update").clone()
}

/// The report a fold would build from one decoded update.
fn report<'a>(update: &'a Value, result_text: Option<&'a str>) -> ToolCallReport<'a> {
    ToolCallReport { id: update.get("toolCallId")
                               .and_then(Value::as_str)
                               .expect("fixture has a toolCallId"),
                     meta: update.get("_meta"),
                     raw_input: update.get("rawInput"),
                     status: update.get("status").and_then(Value::as_str),
                     result_text }
}

fn claude() -> &'static dyn Recognizer {
    for_agent_type("claude").expect("claude has a recognizer")
}

#[test]
fn a_delegation_is_recognized_with_its_kind_and_task() {
    let update = update(DISPATCH);

    let event = claude().recognize(&report(&update, None))
                        .expect("the delegation fixture is recognized");

    assert_eq!(event,
               SubagentEvent::Dispatched { id:   SubagentId::new("toolu_01A1B2C3D4E5F6G7H8J9K0"),
                                           kind: SubagentKind::Named("discovery".to_owned()),
                                           task: "Map persist() callers".to_owned(), });
}

#[test]
fn an_ordinary_tool_call_is_not_a_delegation() {
    let update = update(NOT_A_DELEGATION);

    assert_eq!(claude().recognize(&report(&update, None)), None);
}

#[test]
fn a_completion_is_recognized_as_a_success() {
    let update = update(COMPLETION);

    let event = claude().recognize(&report(&update, None))
                        .expect("the completion fixture is recognized");

    assert_eq!(event,
               SubagentEvent::Completed { id:      SubagentId::new("toolu_01A1B2C3D4E5F6G7H8J9K0"),
                                          outcome: Outcome::Succeeded,
                                          reason:  None, });
}

/// The detail that would have broken this quietly: `claudeCodeMetaFromToolUse`
/// stamps `subagent: true`, and the completion path does not call it - it
/// builds `{ toolName, ...nonExecution }` inline. A recognizer keyed on the
/// flag alone would see every dispatch and no completion.
#[test]
fn the_completion_fixture_carries_no_subagent_flag_and_is_still_recognized() {
    let update = update(COMPLETION);

    assert_eq!(update.pointer("/_meta/claudeCode/subagent"), None);
    assert_eq!(update.pointer("/_meta/claudeCode/toolName")
                     .and_then(Value::as_str),
               Some("Task"));
    assert!(claude().recognize(&report(&update, None)).is_some());
}

#[test]
fn a_failed_completion_keeps_the_result_text_as_its_reason() {
    let update = update(COMPLETION_FAILED);
    let reason = "Agent type 'discovery' not found.";

    let event = claude().recognize(&report(&update, Some(reason)))
                        .expect("the failure fixture is recognized");

    assert_eq!(event,
               SubagentEvent::Completed { id:      SubagentId::new("toolu_01A1B2C3D4E5F6G7H8J9K0"),
                                          outcome: Outcome::Failed,
                                          reason:  Some(reason.to_owned()), });
}

#[test]
fn a_failure_with_no_result_text_is_still_a_failure() {
    let update = update(COMPLETION_FAILED);

    let event = claude().recognize(&report(&update, None))
                        .expect("the failure fixture is recognized");

    let SubagentEvent::Completed { outcome, reason, .. } = event
    else {
        panic!("expected a completion");
    };

    assert_eq!(outcome, Outcome::Failed);
    assert_eq!(reason, None);
}

/// Failing closed on partial input: the delegation is real but its task is
/// gone, so there is nothing worth a row.
#[test]
fn a_delegation_missing_its_description_is_not_recognized() {
    let mut update = update(DISPATCH);
    update["rawInput"].as_object_mut()
                      .expect("rawInput is an object")
                      .remove("description");

    assert_eq!(claude().recognize(&report(&update, None)), None);
}

/// A delegation that names no persona is still a delegation.
#[test]
fn a_delegation_missing_its_subagent_type_is_recognized_as_unstated() {
    let mut update = update(DISPATCH);
    update["rawInput"].as_object_mut()
                      .expect("rawInput is an object")
                      .remove("subagent_type");

    let event = claude().recognize(&report(&update, None))
                        .expect("a delegation with no persona is still one");

    let SubagentEvent::Dispatched { kind, .. } = event
    else {
        panic!("expected a dispatch");
    };

    assert_eq!(kind, SubagentKind::Unstated);
}

/// `Task` and `Agent` are the same tool under two names the adapter has used.
#[test]
fn the_delegation_tool_is_recognized_under_either_name() {
    let mut update = update(COMPLETION);
    update["_meta"]["claudeCode"]["toolName"] = Value::String("Agent".to_owned());

    assert!(claude().recognize(&report(&update, None)).is_some());
}

/// An adapter growing a status word we cannot read must leave the record
/// running and visibly stale, never guess at completion.
#[test]
fn an_unreadable_status_reports_nothing() {
    let mut update = update(COMPLETION);
    update["status"] = Value::String("cancelled".to_owned());

    assert_eq!(claude().recognize(&report(&update, None)), None);
}

/// A tool call with no metadata at all cannot be identified, and identifying
/// it by guesswork is what this recognizer exists to avoid.
#[test]
fn a_call_with_no_metadata_is_not_a_delegation() {
    let mut update = update(DISPATCH);
    update.as_object_mut()
          .expect("update is an object")
          .remove("_meta");

    assert_eq!(claude().recognize(&report(&update, None)), None);
}

/// `kind` is `"think"` for a delegation and shared with ordinary reasoning
/// calls; `title` is prose with an English fallback. Neither may be load
/// bearing, so changing both must change nothing.
#[test]
fn neither_kind_nor_title_is_read() {
    let mut update = update(DISPATCH);
    update["kind"] = Value::String("other".to_owned());
    update["title"] = Value::String("something else entirely".to_owned());

    let event = claude().recognize(&report(&update, None))
                        .expect("still recognized without kind or title");

    let SubagentEvent::Dispatched { task, .. } = event
    else {
        panic!("expected a dispatch");
    };

    assert_eq!(task, "Map persist() callers");
}

#[test]
fn a_type_with_no_recognizer_has_none() {
    assert!(for_agent_type("gemini").is_none());
    assert!(for_agent_type("shell").is_none());
    assert!(for_agent_type("nothing-by-that-name").is_none());
}
