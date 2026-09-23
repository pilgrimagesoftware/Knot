use std::collections::BTreeSet;
use std::sync::Arc;

use knot_mcp_probe::{Inventory, ProbeError, ServerRow, ServerState, Target};
use parking_lot::Mutex;
use uuid::Uuid;

use super::{InFlight, McpSection, ProbeClaim, ProbeResults};

fn slots() -> (InFlight, ProbeResults) {
    (Arc::new(Mutex::new(BTreeSet::new())), Arc::new(Mutex::new(Vec::new())))
}

fn inventory(names: &[&str]) -> Inventory {
    Inventory::probed(names.iter()
                           .map(|name| {
                               ServerRow::new(*name,
                                              Target::Http { url: format!("https://{name}/mcp"), },
                                              ServerState::Connected)
                           })
                           .collect())
}

/// A second request while one is running is dropped, not queued: the running
/// probe's result answers both, and queueing would run the agent's CLI again
/// for no new information.
#[test]
fn a_second_claim_on_the_same_agent_is_refused() {
    let (in_flight, results) = slots();
    let agent = Uuid::new_v4();

    let first = ProbeClaim::claim(&in_flight, &results, agent);
    assert!(first.is_some());
    assert!(ProbeClaim::claim(&in_flight, &results, agent).is_none());

    drop(first);
    assert!(ProbeClaim::claim(&in_flight, &results, agent).is_some(),
            "the slot frees once the first finishes");
}

#[test]
fn different_agents_probe_independently() {
    let (in_flight, results) = slots();

    let a = ProbeClaim::claim(&in_flight, &results, Uuid::new_v4());
    let b = ProbeClaim::claim(&in_flight, &results, Uuid::new_v4());

    assert!(a.is_some() && b.is_some());
}

#[test]
fn reporting_leaves_the_outcome_and_frees_the_slot() {
    let (in_flight, results) = slots();
    let agent = Uuid::new_v4();

    ProbeClaim::claim(&in_flight, &results, agent).expect("free")
                                                  .report(Ok(inventory(&["github"])));

    assert!(in_flight.lock().is_empty());
    assert_eq!(results.lock().len(), 1);
    assert!(results.lock()[0].1.is_ok());
}

/// The #376 shape, per agent. A claim dropped without reporting used to leave
/// the flag raised forever; here it must both free the slot *and* leave an
/// answer, or the header sits on "not checked yet" while nothing will check.
#[test]
fn a_claim_dropped_without_reporting_still_answers() {
    let (in_flight, results) = slots();
    let agent = Uuid::new_v4();

    drop(ProbeClaim::claim(&in_flight, &results, agent).expect("free"));

    assert!(in_flight.lock().is_empty(),
            "the slot must free on an unwind");
    assert_eq!(results.lock().len(),
               1,
               "the section must still get an answer");
    assert!(results.lock()[0].1.is_err());
}

/// Peeking must not consume. A caller that took the request and then
/// returned early - because the agent vanished from the store for a frame,
/// say - would lose it permanently: the section has asked once, so nothing
/// asks again, and it sits on "not checked yet" while nothing will ever
/// check. The symptom is an absence, which no ordinary test would catch.
#[test]
fn peeking_at_a_request_does_not_take_it() {
    let mut section = McpSection::default();

    section.request();

    assert!(section.wants_probe());
    assert!(section.wants_probe(), "peeking twice must still see it");
    assert!(section.take_request(),
            "the request survived being looked at");
    assert!(!section.wants_probe());
}

#[test]
fn a_new_section_wants_its_first_probe() {
    let mut section = McpSection::default();

    assert!(section.needs_first_probe());
    section.request();
    assert!(!section.needs_first_probe(), "already asked");

    assert!(section.take_request());
    assert!(!section.take_request(), "a request is taken once");
    assert!(!section.needs_first_probe(),
            "becoming visible again must not re-ask on its own");
}

/// The failure that must not blank the list. Rows carry their own timestamp,
/// so showing both what was last known and why it could not be rechecked is
/// strictly more useful than showing neither.
#[test]
fn a_failure_keeps_the_rows_that_landed_before_it() {
    let mut section = McpSection::default();

    section.publish(Ok(inventory(&["github", "sentry"])));
    assert_eq!(section.inventory().map(|i| i.rows().len()), Some(2));
    assert_eq!(section.failure(), None);

    section.publish(Err(ProbeError::TimedOut { program: "claude mcp list".to_owned(),
                                               seconds: 30, }));

    assert_eq!(section.inventory().map(|i| i.rows().len()),
               Some(2),
               "the earlier rows were discarded");
    assert!(section.failure()
                   .is_some_and(|text| text.contains("timed out")));
}

#[test]
fn a_success_clears_an_earlier_failure() {
    let mut section = McpSection::default();

    section.publish(Err(ProbeError::Missing { program: "claude".to_owned(), }));
    assert!(section.failure().is_some());

    section.publish(Ok(inventory(&["github"])));
    assert_eq!(section.failure(),
               None,
               "a stale error must not outlive the probe that fixed it");
}

#[test]
fn toggling_opens_and_shuts() {
    let mut section = McpSection::default();

    assert!(!section.expanded);
    assert!(section.toggle());
    assert!(!section.toggle());
}

/// Probing is not gated on the disclosure state: the collapsed header names
/// what needs attention, so it needs an answer just as much as the open one.
#[test]
fn collapsing_does_not_cancel_a_request() {
    let mut section = McpSection::default();

    section.request();
    section.toggle();
    section.toggle();

    assert!(section.take_request(),
            "the request survived being opened and shut");
}
