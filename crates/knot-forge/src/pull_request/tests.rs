//! Unit tests for [`super`].

use super::{CheckRollup, PullRequestStatus, parse_pull_request_state, pull_request_state_with};
use crate::error::ForgeError;
use crate::runner::stub::StubRunner;

/// Captured verbatim from `gh pr view --json
/// number,title,state,isDraft,statusCheckRollup` against this repository's own
/// pull request #314, so the shape under test is the shape `gh` actually
/// sends - including the fields Knot does not ask about but is given anyway.
const CAPTURED_OPEN: &str = include_str!("testdata/open-passing.json");

/// The same, against merged pull request #312.
const CAPTURED_MERGED: &str = include_str!("testdata/merged-passing.json");

#[test]
fn a_captured_open_payload_parses() {
    let state = parse_pull_request_state(CAPTURED_OPEN).unwrap();

    assert_eq!(state.number, Some(314));
    assert_eq!(state.title.as_deref(),
               Some("Record the pull requests agents open, and list them per workspace"));
    assert_eq!(state.status, PullRequestStatus::Open);
    assert_eq!(state.checks, Some(CheckRollup::Passing));
}

#[test]
fn a_captured_merged_payload_parses() {
    let state = parse_pull_request_state(CAPTURED_MERGED).unwrap();

    assert_eq!(state.number, Some(312));
    assert_eq!(state.status, PullRequestStatus::Merged);
    assert_eq!(state.checks, Some(CheckRollup::Passing));
}

/// GitHub models draft as `state: OPEN` plus `isDraft`. The row shows one
/// thing, so the flattening happens here and is worth pinning.
#[test]
fn an_open_pull_request_marked_draft_is_draft() {
    let json = r#"{"number":7,"title":"wip","state":"OPEN","isDraft":true,
                   "statusCheckRollup":[]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().status,
               PullRequestStatus::Draft);
}

#[test]
fn a_closed_pull_request_is_closed() {
    let json = r#"{"number":7,"title":"no","state":"CLOSED","isDraft":false}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().status,
               PullRequestStatus::Closed);
}

#[test]
fn draft_and_open_are_both_open_for_counting() {
    assert!(PullRequestStatus::Draft.is_open());
    assert!(PullRequestStatus::Open.is_open());
    assert!(!PullRequestStatus::Merged.is_open());
    assert!(!PullRequestStatus::Closed.is_open());
}

// --- Tolerance -------------------------------------------------------------

/// `gh` volunteers fields Knot never asked for and adds more between
/// versions. One of them is not a reason to lose the row.
#[test]
fn an_unknown_extra_field_does_not_fail_the_parse() {
    let json = r#"{"number":7,"title":"t","state":"OPEN","isDraft":false,
                   "someFieldFromANewerGh":{"nested":[1,2,3]},"mergeable":"MERGEABLE"}"#;

    let state = parse_pull_request_state(json).unwrap();

    assert_eq!(state.number, Some(7));
    assert_eq!(state.status, PullRequestStatus::Open);
}

/// A `gh` too old to report a field degrades that part of the row. The row
/// still lists, still shows its status, and still opens.
#[test]
fn a_missing_field_leaves_that_part_absent_rather_than_failing() {
    let state = parse_pull_request_state(r#"{"state":"OPEN"}"#).unwrap();

    assert_eq!(state.status, PullRequestStatus::Open);
    assert_eq!(state.number, None);
    assert_eq!(state.title, None);
    assert_eq!(state.checks, None);
}

/// The one field that cannot be absent: a row that does not know whether its
/// pull request is open would have to guess, and guessing "open" shows a
/// merged pull request as outstanding.
#[test]
fn a_missing_state_is_a_parse_failure() {
    match parse_pull_request_state(r#"{"number":7,"title":"t"}"#) {
        Err(ForgeError::Parse(_)) => {}
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn an_unknown_state_is_a_parse_failure_rather_than_a_guess() {
    match parse_pull_request_state(r#"{"state":"ASCENDED"}"#) {
        Err(ForgeError::Parse(reason)) => assert!(reason.contains("ASCENDED"), "{reason}"),
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn malformed_json_is_a_parse_failure() {
    assert!(matches!(parse_pull_request_state("not json"),
                     Err(ForgeError::Parse(_))));
}

// --- The check rollup ------------------------------------------------------

#[test]
fn any_failing_check_fails_the_rollup() {
    let json = r#"{"state":"OPEN","statusCheckRollup":[
        {"status":"COMPLETED","conclusion":"SUCCESS"},
        {"status":"COMPLETED","conclusion":"FAILURE"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().checks,
               Some(CheckRollup::Failing));
}

#[test]
fn an_unfinished_check_makes_the_rollup_pending() {
    let json = r#"{"state":"OPEN","statusCheckRollup":[
        {"status":"COMPLETED","conclusion":"SUCCESS"},
        {"status":"IN_PROGRESS"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().checks,
               Some(CheckRollup::Pending));
}

/// A commit status rides in the same array as a check run and reports
/// `state` where a check run reports `conclusion`.
#[test]
fn a_commit_status_is_read_alongside_check_runs() {
    let json = r#"{"state":"OPEN","statusCheckRollup":[
        {"state":"FAILURE","context":"ci/external"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().checks,
               Some(CheckRollup::Failing));
}

/// No checks configured is not a result. A green tick nobody earned is worse
/// than nothing.
#[test]
fn no_checks_is_absent_rather_than_passing() {
    let json = r#"{"state":"OPEN","statusCheckRollup":[]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().checks, None);
}

// --- The call ---------------------------------------------------------------

#[test]
fn the_url_is_passed_to_gh_with_the_fields_the_view_needs() {
    let runner = StubRunner::ok(CAPTURED_OPEN);

    pull_request_state_with(&runner, "https://github.com/acme/widget/pull/42").unwrap();

    assert_eq!(runner.calls(),
               vec!["pr view https://github.com/acme/widget/pull/42 --json \
                     number,title,state,isDraft,statusCheckRollup"]);
}

#[test]
fn a_missing_binary_surfaces_as_missing() {
    let err = pull_request_state_with(&StubRunner::missing(), "https://github.com/a/b/pull/1")
        .unwrap_err();

    assert!(matches!(err, ForgeError::Missing));
    assert!(err.is_persistent());
}

/// A pull request that has been deleted, or a URL `gh` cannot resolve, is a
/// failed fetch - which the spec covers without removing the record.
#[test]
fn a_failed_lookup_surfaces_as_a_command_failure() {
    let runner = StubRunner::failing("could not resolve to a PullRequest", 1);

    let err = pull_request_state_with(&runner, "https://github.com/a/b/pull/1").unwrap_err();

    assert!(matches!(err, ForgeError::Command { .. }));
    assert!(!err.is_persistent(), "a failed lookup is worth retrying");
}
