use super::SubagentEvent;
use crate::kind::SubagentKind;
use crate::state::Outcome;
use crate::subagent::SubagentId;

#[test]
fn a_dispatch_carries_the_kind_and_task_it_was_given() {
    let event = SubagentEvent::dispatched("call-1", Some("discovery"), Some("Map the callers"))
        .expect("a dispatch with a task is recognized");

    assert_eq!(event,
               SubagentEvent::Dispatched { id:   SubagentId::new("call-1"),
                                           kind: SubagentKind::Named("discovery".to_owned()),
                                           task: "Map the callers".to_owned(), });
}

#[test]
fn a_dispatch_without_a_kind_is_still_a_dispatch() {
    let event = SubagentEvent::dispatched("call-1", None, Some("Map the callers"))
        .expect("a dispatch with no kind is recognized");

    let SubagentEvent::Dispatched { kind, .. } = &event
    else {
        panic!("expected a dispatch");
    };

    assert_eq!(*kind, SubagentKind::Unstated);
}

/// Failing closed. A row asserting that something is running without saying
/// what tells the user less than no row at all.
#[test]
fn a_dispatch_with_no_task_is_not_recognized() {
    assert_eq!(SubagentEvent::dispatched("call-1", Some("discovery"), None),
               None);
    assert_eq!(SubagentEvent::dispatched("call-1", Some("discovery"), Some("")),
               None);
    assert_eq!(SubagentEvent::dispatched("call-1", Some("discovery"), Some("  \n ")),
               None);
}

#[test]
fn a_completion_parses_its_outcome_word() {
    let event =
        SubagentEvent::completed("call-1", "succeeded", None).expect("a known outcome parses");

    assert_eq!(event,
               SubagentEvent::Completed { id:      SubagentId::new("call-1"),
                                          outcome: Outcome::Succeeded,
                                          reason:  None, });
}

#[test]
fn a_failed_completion_keeps_its_reason() {
    let event = SubagentEvent::completed("call-1", "failed", Some("ran out of context"))
        .expect("a known outcome parses");

    assert_eq!(event,
               SubagentEvent::Completed { id:      SubagentId::new("call-1"),
                                          outcome: Outcome::Failed,
                                          reason:  Some("ran out of context".to_owned()), });
}

/// An adapter growing a third outcome word must leave the record visibly
/// running rather than silently marking it finished.
#[test]
fn an_unknown_outcome_is_not_recognized_as_success() {
    assert_eq!(SubagentEvent::completed("call-1", "cancelled", None), None);
    assert_eq!(SubagentEvent::completed("call-1", "", None), None);
}

#[test]
fn a_blank_reason_is_dropped_rather_than_stored_as_empty() {
    let event =
        SubagentEvent::completed("call-1", "failed", Some("   ")).expect("the outcome parses");

    let SubagentEvent::Completed { reason, .. } = &event
    else {
        panic!("expected a completion");
    };

    assert_eq!(*reason, None);
}

/// Both variants answer the same question, which is what lets the registry
/// look a record up before it decides what to do with the event.
#[test]
fn both_variants_name_their_subagent() {
    let dispatched = SubagentEvent::dispatched("call-7", None, Some("go")).expect("recognized");
    let completed = SubagentEvent::completed("call-7", "succeeded", None).expect("recognized");

    assert_eq!(dispatched.id(), &SubagentId::new("call-7"));
    assert_eq!(completed.id(), &SubagentId::new("call-7"));
}
