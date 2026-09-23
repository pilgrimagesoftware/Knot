//! The closed-vocabulary guarantee for [`SubagentState`] and [`Outcome`].
//!
//! `every_state` is written out by hand on purpose. A variant added to the
//! enum and forgotten here does not fail *this* file - it fails `as_str` and
//! `failure_reason` in the parent, which match exhaustively with no `_` arm.
//! That is the guarantee worth having: the compiler finds the next site, and
//! these tests then prove the new word round-trips rather than silently
//! resolving to another variant.

use std::str::FromStr;

use super::{Outcome, SubagentState};
use crate::error::SubagentError;

fn every_state() -> Vec<SubagentState> {
    vec![SubagentState::Running,
         SubagentState::Finished,
         SubagentState::Failed { reason: None },
         SubagentState::Failed { reason: Some("the subagent ran out of context".to_owned()), }]
}

#[test]
fn every_state_round_trips_through_its_word() {
    for state in every_state() {
        let parsed = SubagentState::from_str(state.as_str()).expect("state word parses");

        assert_eq!(parsed.as_str(), state.as_str());
    }
}

#[test]
fn every_state_has_a_distinct_word_except_the_two_failures() {
    let words: Vec<&str> = every_state().iter().map(SubagentState::as_str).collect();

    assert_eq!(words, vec!["running", "finished", "failed", "failed"]);
}

#[test]
fn an_unknown_word_is_an_error_not_a_default() {
    let parsed = SubagentState::from_str("pending");

    assert_eq!(parsed,
               Err(SubagentError::UnknownState("pending".to_owned())));
}

#[test]
fn parsing_failed_yields_no_reason_rather_than_an_empty_one() {
    let parsed = SubagentState::from_str("failed").expect("failed parses");

    assert_eq!(parsed, SubagentState::Failed { reason: None });
    assert_eq!(parsed.failure_reason(), None);
}

#[test]
fn a_failure_keeps_the_reason_it_was_built_with() {
    let state = SubagentState::Failed { reason: Some("no such persona".to_owned()), };

    assert_eq!(state.failure_reason(), Some("no such persona"));
}

#[test]
fn only_running_is_running() {
    assert!(SubagentState::Running.is_running());
    assert!(!SubagentState::Finished.is_running());
    assert!(!SubagentState::Failed { reason: None }.is_running());
}

#[test]
fn a_finished_state_has_no_failure_reason() {
    assert_eq!(SubagentState::Finished.failure_reason(), None);
    assert_eq!(SubagentState::Running.failure_reason(), None);
}

#[test]
fn every_outcome_round_trips() {
    for outcome in Outcome::ALL {
        let parsed = Outcome::from_str(outcome.as_str()).expect("outcome word parses");

        assert_eq!(parsed, *outcome);
    }
}

#[test]
fn an_unknown_outcome_is_an_error_not_a_success() {
    let parsed = Outcome::from_str("cancelled");

    assert_eq!(parsed,
               Err(SubagentError::UnknownOutcome("cancelled".to_owned())));
}

#[test]
fn a_failed_outcome_carries_its_reason_into_the_state() {
    let state = Outcome::Failed.into_state(Some("timed out".to_owned()));

    assert_eq!(state,
               SubagentState::Failed { reason: Some("timed out".to_owned()), });
}

#[test]
fn a_succeeded_outcome_drops_a_reason_rather_than_failing() {
    let state = Outcome::Succeeded.into_state(Some("ignored".to_owned()));

    assert_eq!(state, SubagentState::Finished);
}
