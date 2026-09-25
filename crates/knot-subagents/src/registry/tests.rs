//! Registry behaviour, driven by a supplied clock so nothing sleeps.

use std::time::{Duration, Instant};

use uuid::Uuid;

use super::SubagentRegistry;
use crate::consts::MAX_FINISHED_PER_AGENT;
use crate::event::SubagentEvent;
use crate::kind::SubagentKind;
use crate::state::{Outcome, SubagentState};
use crate::subagent::SubagentId;

fn dispatch(id: &str, kind: &str) -> SubagentEvent {
    SubagentEvent::Dispatched { id:   SubagentId::new(id),
                                kind: SubagentKind::Named(kind.to_owned()),
                                task: format!("task for {id}"), }
}

fn complete(id: &str, outcome: Outcome) -> SubagentEvent {
    SubagentEvent::Completed { id: SubagentId::new(id),
                               outcome,
                               reason: None }
}

fn secs(base: Instant, n: u64) -> Instant {
    base + Duration::from_secs(n)
}

#[test]
fn a_dispatch_is_recorded_against_its_agent() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, dispatch("s1", "discovery"), base);

    let records = registry.ordered(agent, secs(base, 1));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, SubagentKind::Named("discovery".to_owned()));
    assert!(records[0].is_running());
}

#[test]
fn a_completion_moves_the_record_it_names() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, dispatch("s1", "discovery"), base);
    registry.apply(agent, complete("s1", Outcome::Succeeded), secs(base, 30));

    let records = registry.ordered(agent, secs(base, 60));
    assert_eq!(records[0].state, SubagentState::Finished);
    assert_eq!(records[0].elapsed(secs(base, 600)), Duration::from_secs(30));
}

/// Knot may have started mid-turn. Inventing a finished row for a subagent
/// whose task and kind are unknown would put a blank line on screen.
#[test]
fn a_completion_for_an_unknown_subagent_creates_nothing() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, complete("never-seen", Outcome::Succeeded), base);

    assert!(registry.is_empty_for(agent));
    assert!(registry.ordered(agent, base).is_empty());
}

#[test]
fn a_completion_for_another_agents_subagent_does_nothing() {
    let (one, two, base) = (Uuid::new_v4(), Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(one, dispatch("s1", "discovery"), base);
    registry.apply(two, complete("s1", Outcome::Succeeded), secs(base, 5));

    assert!(registry.ordered(one, secs(base, 10))[0].is_running());
    assert!(registry.is_empty_for(two));
}

/// The adapter refines a call it already announced. That is not a second
/// subagent, and it must not reset the first one's clock.
#[test]
fn a_repeat_dispatch_keeps_the_original_start() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, dispatch("s1", "discovery"), base);
    registry.apply(agent, dispatch("s1", "discovery"), secs(base, 100));

    let records = registry.ordered(agent, secs(base, 100));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].elapsed(secs(base, 100)),
               Duration::from_secs(100));
}

/// The spec's ordering scenario: two running and one finished.
#[test]
fn records_order_running_first_then_longest_running() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, dispatch("older", "a"), base);
    registry.apply(agent, dispatch("newer", "b"), secs(base, 50));
    registry.apply(agent, dispatch("done", "c"), secs(base, 10));
    registry.apply(agent, complete("done", Outcome::Succeeded), secs(base, 20));

    let ids: Vec<&str> = registry.ordered(agent, secs(base, 100))
                                 .iter()
                                 .map(|record| record.id.as_str())
                                 .collect();

    assert_eq!(ids, vec!["older", "newer", "done"]);
}

#[test]
fn clearing_forgets_one_agent_and_leaves_the_others() {
    let (one, two, base) = (Uuid::new_v4(), Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(one, dispatch("s1", "a"), base);
    registry.apply(two, dispatch("s2", "b"), base);

    registry.clear(one);

    assert!(registry.is_empty_for(one));
    assert_eq!(registry.ordered(two, base).len(), 1);
}

#[test]
fn running_records_are_counted_separately() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, dispatch("s1", "a"), base);
    registry.apply(agent, dispatch("s2", "b"), base);
    registry.apply(agent, complete("s1", Outcome::Failed), secs(base, 5));

    assert_eq!(registry.running_count(agent), 1);
    assert_eq!(registry.ordered(agent, secs(base, 10)).len(), 2);
}

#[test]
fn finished_records_past_the_cap_drop_oldest_first() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    for n in 0..=MAX_FINISHED_PER_AGENT {
        let id = format!("s{n}");
        registry.apply(agent, dispatch(&id, "a"), base);
        // Each finishes one second after the last, so "oldest" is unambiguous.
        registry.apply(agent,
                       complete(&id, Outcome::Succeeded),
                       secs(base, n as u64 + 1));
    }

    let ids: Vec<String> = registry.ordered(agent, secs(base, 1000))
                                   .iter()
                                   .map(|record| record.id.as_str().to_owned())
                                   .collect();

    assert_eq!(ids.len(), MAX_FINISHED_PER_AGENT);
    assert!(!ids.contains(&"s0".to_owned()),
            "the oldest should have been dropped");
    assert!(ids.contains(&format!("s{MAX_FINISHED_PER_AGENT}")),
            "the newest should have been kept");
}

/// The cap may only ever hide work that is already over.
#[test]
fn running_records_are_never_dropped_however_many_there_are() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    for n in 0..MAX_FINISHED_PER_AGENT * 2 {
        registry.apply(agent, dispatch(&format!("s{n}"), "a"), base);
    }
    // One completion, to make the prune run at all.
    registry.apply(agent, dispatch("done", "a"), base);
    registry.apply(agent, complete("done", Outcome::Succeeded), secs(base, 1));

    assert_eq!(registry.running_count(agent), MAX_FINISHED_PER_AGENT * 2);
}

#[test]
fn a_change_is_reported_once_and_then_cleared() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    assert!(!registry.take_changed(), "a fresh registry has no news");

    registry.apply(agent, dispatch("s1", "a"), base);

    assert!(registry.take_changed());
    assert!(!registry.take_changed(), "the flag is consumed by the read");
}

#[test]
fn a_completion_and_a_clear_each_report_a_change() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, dispatch("s1", "a"), base);
    registry.take_changed();

    registry.apply(agent, complete("s1", Outcome::Succeeded), secs(base, 1));
    assert!(registry.take_changed(), "a completion is news");

    registry.clear(agent);
    assert!(registry.take_changed(), "a clear is news");
}

/// A dropped event must not ask for a repaint: notifying on one would repaint
/// at the feed's rate whether anything moved or not.
#[test]
fn an_ignored_event_reports_no_change() {
    let (agent, base) = (Uuid::new_v4(), Instant::now());
    let mut registry = SubagentRegistry::new();

    registry.apply(agent, complete("never-seen", Outcome::Succeeded), base);
    assert!(!registry.take_changed());

    registry.clear(agent);
    assert!(!registry.take_changed(),
            "clearing an agent with nothing is not news");
}
