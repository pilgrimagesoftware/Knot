//! Elapsed-time arithmetic, driven by a supplied clock rather than a sleep.
//!
//! Every instant here is derived from one `base` by addition, so the tests
//! assert exact durations and cannot flake on a slow machine. That is the
//! whole reason `elapsed` takes `now` as an argument.

use std::time::{Duration, Instant};

use super::{Subagent, SubagentId};
use crate::kind::SubagentKind;
use crate::state::{Outcome, SubagentState};

fn dispatched(started: Instant) -> Subagent {
    Subagent::dispatched(SubagentId::new("call-1"),
                         SubagentKind::from_reported(Some("discovery")),
                         "Find every caller of persist()".to_owned(),
                         started)
}

#[test]
fn a_dispatched_subagent_starts_running_with_no_end() {
    let subagent = dispatched(Instant::now());

    assert_eq!(subagent.state, SubagentState::Running);
    assert_eq!(subagent.ended, None);
    assert!(subagent.is_running());
}

#[test]
fn a_running_subagents_elapsed_advances_with_the_clock() {
    let base = Instant::now();
    let subagent = dispatched(base);

    assert_eq!(subagent.elapsed(base + Duration::from_secs(60)),
               Duration::from_secs(60));
    assert_eq!(subagent.elapsed(base + Duration::from_secs(240)),
               Duration::from_secs(240));
}

#[test]
fn a_finished_subagents_elapsed_stops_at_its_end() {
    let base = Instant::now();
    let mut subagent = dispatched(base);

    subagent.complete(Outcome::Succeeded, None, base + Duration::from_secs(90));

    // The clock keeps moving; the record does not.
    assert_eq!(subagent.elapsed(base + Duration::from_secs(90)),
               Duration::from_secs(90));
    assert_eq!(subagent.elapsed(base + Duration::from_secs(3600)),
               Duration::from_secs(90));
}

#[test]
fn completing_records_the_outcome_and_its_reason() {
    let base = Instant::now();
    let mut subagent = dispatched(base);

    subagent.complete(Outcome::Failed,
                      Some("ran out of context".to_owned()),
                      base + Duration::from_secs(5));

    assert_eq!(subagent.state,
               SubagentState::Failed { reason: Some("ran out of context".to_owned()), });
    assert!(!subagent.is_running());
}

/// A duplicate completion must not stretch a finished subagent's elapsed time,
/// which is what would happen if the second report overwrote the end instant.
#[test]
fn a_second_completion_keeps_the_first_end_instant() {
    let base = Instant::now();
    let mut subagent = dispatched(base);

    subagent.complete(Outcome::Succeeded, None, base + Duration::from_secs(10));
    subagent.complete(Outcome::Succeeded, None, base + Duration::from_secs(400));

    assert_eq!(subagent.elapsed(base + Duration::from_secs(900)),
               Duration::from_secs(10));
}

/// A render pass reads the clock on the main thread while the record was
/// stamped on another. Saturating is what keeps that from panicking a frame.
#[test]
fn a_now_before_the_start_saturates_to_zero_rather_than_panicking() {
    let base = Instant::now();
    let subagent = dispatched(base + Duration::from_secs(30));

    assert_eq!(subagent.elapsed(base), Duration::ZERO);
}

#[test]
fn the_record_keeps_the_task_verbatim() {
    let subagent = dispatched(Instant::now());

    assert_eq!(subagent.task, "Find every caller of persist()");
}

#[test]
fn an_id_round_trips_through_its_string() {
    let id = SubagentId::new("toolu_01ABC");

    assert_eq!(id.as_str(), "toolu_01ABC");
    assert_eq!(id.to_string(), "toolu_01ABC");
}
