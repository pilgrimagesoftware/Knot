//! The hook payload shape, asserted from this side because Knot defines it.
//!
//! Unlike the Claude ACP recognizer, there is no external contract to capture
//! here: the emitting plugin is written against *this*, not the other way
//! round. These tests are therefore the specification of the payload, and the
//! JSON is written inline on purpose.

use serde_json::json;

use super::{
    HOOK_SUBAGENT_START, HOOK_SUBAGENT_STOP, is_subagent_hook, recognize_hook,
    subagent_payload_is_complete,
};
use crate::event::SubagentEvent;
use crate::kind::SubagentKind;
use crate::state::Outcome;
use crate::subagent::SubagentId;

#[test]
fn a_dispatch_payload_is_recognized() {
    let payload = json!({
        "subagent_id": "sub-1",
        "subagent_type": "discovery",
        "task": "Map persist() callers",
    });

    let event = recognize_hook(HOOK_SUBAGENT_START, &payload).expect("recognized");

    assert_eq!(event,
               SubagentEvent::Dispatched { id:   SubagentId::new("sub-1"),
                                           kind: SubagentKind::Named("discovery".to_owned()),
                                           task: "Map persist() callers".to_owned(), });
}

#[test]
fn a_dispatch_without_a_type_is_recognized_as_unstated() {
    let payload = json!({ "subagent_id": "sub-1", "task": "Map persist() callers" });

    let event = recognize_hook(HOOK_SUBAGENT_START, &payload).expect("recognized");

    let SubagentEvent::Dispatched { kind, .. } = event
    else {
        panic!("expected a dispatch");
    };

    assert_eq!(kind, SubagentKind::Unstated);
    assert!(subagent_payload_is_complete(HOOK_SUBAGENT_START, &payload));
}

#[test]
fn a_completion_payload_is_recognized() {
    let payload = json!({ "subagent_id": "sub-1", "outcome": "succeeded" });

    let event = recognize_hook(HOOK_SUBAGENT_STOP, &payload).expect("recognized");

    assert_eq!(event,
               SubagentEvent::Completed { id:      SubagentId::new("sub-1"),
                                          outcome: Outcome::Succeeded,
                                          reason:  None, });
}

#[test]
fn a_failed_completion_keeps_its_reason() {
    let payload = json!({
        "subagent_id": "sub-1",
        "outcome": "failed",
        "reason": "ran out of context",
    });

    let event = recognize_hook(HOOK_SUBAGENT_STOP, &payload).expect("recognized");

    assert_eq!(event,
               SubagentEvent::Completed { id:      SubagentId::new("sub-1"),
                                          outcome: Outcome::Failed,
                                          reason:  Some("ran out of context".to_owned()), });
}

/// The route owes a 400 for these, which is why completeness is asked
/// separately from recognition - `None` alone cannot say whether the payload
/// was wrong or the hook was simply not ours.
#[test]
fn a_dispatch_missing_its_identifier_or_task_is_incomplete() {
    let no_id = json!({ "subagent_type": "discovery", "task": "Map the callers" });
    let no_task = json!({ "subagent_id": "sub-1", "subagent_type": "discovery" });

    assert_eq!(recognize_hook(HOOK_SUBAGENT_START, &no_id), None);
    assert_eq!(recognize_hook(HOOK_SUBAGENT_START, &no_task), None);
    assert!(!subagent_payload_is_complete(HOOK_SUBAGENT_START, &no_id));
    assert!(!subagent_payload_is_complete(HOOK_SUBAGENT_START, &no_task));
}

#[test]
fn a_completion_missing_its_identifier_or_outcome_is_incomplete() {
    let no_id = json!({ "outcome": "succeeded" });
    let no_outcome = json!({ "subagent_id": "sub-1" });

    assert_eq!(recognize_hook(HOOK_SUBAGENT_STOP, &no_id), None);
    assert_eq!(recognize_hook(HOOK_SUBAGENT_STOP, &no_outcome), None);
    assert!(!subagent_payload_is_complete(HOOK_SUBAGENT_STOP, &no_id));
    assert!(!subagent_payload_is_complete(HOOK_SUBAGENT_STOP, &no_outcome));
}

#[test]
fn a_blank_field_counts_as_absent() {
    let payload = json!({ "subagent_id": "sub-1", "task": "   " });

    assert_eq!(recognize_hook(HOOK_SUBAGENT_START, &payload), None);
    assert!(!subagent_payload_is_complete(HOOK_SUBAGENT_START, &payload));
}

#[test]
fn an_unreadable_outcome_is_not_recognized_as_success() {
    let payload = json!({ "subagent_id": "sub-1", "outcome": "cancelled" });

    assert_eq!(recognize_hook(HOOK_SUBAGENT_STOP, &payload), None);
}

/// The route asks this before anything else: a subagent event must not move
/// the agent's activity status, so it has to be told apart from a status post
/// before either is handled.
#[test]
fn only_the_two_lifecycle_events_are_subagent_hooks() {
    assert!(is_subagent_hook(HOOK_SUBAGENT_START));
    assert!(is_subagent_hook(HOOK_SUBAGENT_STOP));

    assert!(!is_subagent_hook("Stop"));
    assert!(!is_subagent_hook("SessionStart"));
    assert!(!is_subagent_hook("PostToolUse"));
}

#[test]
fn an_unknown_hook_reports_nothing_even_with_a_usable_payload() {
    let payload = json!({ "subagent_id": "sub-1", "task": "Map the callers" });

    assert_eq!(recognize_hook("PostToolUse", &payload), None);
    assert!(!subagent_payload_is_complete("PostToolUse", &payload));
}
