//! Unit tests for [`super`].

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{
    CheckRollup, Mergeability, PullRequestStatus, parse_pull_request_state, pull_request_state_with,
};
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
    assert_eq!(state.merged_at,
               Some(OffsetDateTime::parse("2026-09-22T18:23:05Z", &Rfc3339).unwrap()));
}

/// The timestamp the retention window is measured against. Absent from what
/// any row shows, so nothing else in the parse would notice it going missing.
#[test]
fn a_merged_payload_carries_its_merge_time() {
    let state = parse_pull_request_state(
        r#"{"state":"MERGED","number":7,"mergedAt":"2026-01-02T03:04:05Z"}"#,
    ).unwrap();

    assert_eq!(state.merged_at,
               Some(OffsetDateTime::parse("2026-01-02T03:04:05Z", &Rfc3339).unwrap()));
}

/// A `gh` too old to report the field. The record simply never expires, which
/// is the documented degradation - it must not cost the row.
#[test]
fn a_merged_payload_without_a_merge_time_parses_with_none() {
    let state = parse_pull_request_state(r#"{"state":"MERGED","number":7}"#).unwrap();

    assert_eq!(state.status, PullRequestStatus::Merged);
    assert_eq!(state.merged_at, None);
}

/// Dropped rather than raised: losing the whole row over one unreadable field
/// would be worse than a record that does not expire.
#[test]
fn an_unparseable_merge_time_is_dropped_not_fatal() {
    let state =
        parse_pull_request_state(r#"{"state":"MERGED","number":7,"mergedAt":"last Tuesday"}"#)
            .unwrap();

    assert_eq!(state.status, PullRequestStatus::Merged);
    assert_eq!(state.merged_at, None);
}

/// Gated on the status, so a forge volunteering the field on something it also
/// calls closed cannot produce a state that is both merged and not.
#[test]
fn an_unmerged_pull_request_has_no_merge_time() {
    for payload in [r#"{"state":"OPEN","number":7}"#,
                    r#"{"state":"OPEN","number":7,"isDraft":true}"#,
                    r#"{"state":"CLOSED","number":7,"mergedAt":"2026-01-02T03:04:05Z"}"#]
    {
        let state = parse_pull_request_state(payload).unwrap();

        assert_eq!(state.merged_at, None, "{payload}");
    }
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
                     number,title,state,isDraft,mergeable,mergeStateStatus,statusCheckRollup,\
                     mergedAt"],
               "asked once: the captured payload already knows its mergeability");
}

#[test]
fn a_missing_binary_surfaces_as_missing() {
    let err = pull_request_state_with(&StubRunner::missing(), "https://github.com/a/b/pull/1")
        .unwrap_err();

    assert!(matches!(err, ForgeError::Missing));
    assert!(err.is_persistent());
}

/// Both of the forge's "does not exist" answers, verbatim from the live `gh`.
/// A finished answer rather than a failed fetch, so the row can say so
/// instead of waiting forever.
#[test]
fn a_pull_request_the_forge_cannot_resolve_is_not_found() {
    for output in ["GraphQL: Could not resolve to a Repository with the name 'acme/gone'. \
                    (repository)\n",
                   "GraphQL: Could not resolve to a PullRequest with the number of 99999. \
                    (repository.pullRequest)\n"]
    {
        let runner = StubRunner::failing(output, 1);

        let err = pull_request_state_with(&runner, "https://github.com/acme/gone/pull/99999")
            .unwrap_err();

        let ForgeError::NotFound(said) = &err
        else {
            panic!("{output:?} classified as {err:?}");
        };
        assert!(said.contains("Could not resolve"), "{said}");
        assert!(!err.is_persistent(),
                "one missing pull request is not the view's problem");
    }
}

/// The capitalisation is GitHub's to change.
#[test]
fn the_not_found_answer_is_matched_regardless_of_case() {
    let runner = StubRunner::failing("could not resolve to a pullrequest with the number of 3", 1);

    let err = pull_request_state_with(&runner, "https://github.com/a/b/pull/3").unwrap_err();

    assert!(matches!(err, ForgeError::NotFound(_)), "{err:?}");
}

/// Any other failure stays a failed fetch, which is retried. Reading a network
/// error as "does not exist" would stop a real pull request refreshing.
#[test]
fn any_other_command_failure_stays_a_command_failure() {
    let runner = StubRunner::failing("error connecting to api.github.com", 1);

    let err = pull_request_state_with(&runner, "https://github.com/a/b/pull/1").unwrap_err();

    assert!(matches!(err, ForgeError::Command { .. }), "{err:?}");
    assert!(!err.is_persistent(), "a failed lookup is worth retrying");
}

// --- Mergeability -----------------------------------------------------------

/// The captured open pull request is mergeable, so its row earns a colour
/// from one fetch.
#[test]
fn a_captured_mergeable_pull_request_reads_as_mergeable() {
    let state = parse_pull_request_state(CAPTURED_OPEN).unwrap();

    assert_eq!(state.mergeable, Mergeability::Mergeable);
}

/// The question does not arise once it is merged or closed, and GitHub
/// answers `UNKNOWN` there anyway.
#[test]
fn a_pull_request_that_is_not_open_has_no_mergeability() {
    assert_eq!(parse_pull_request_state(CAPTURED_MERGED).unwrap().mergeable,
               Mergeability::Unknown);

    let closed = r#"{"state":"CLOSED","mergeable":"MERGEABLE"}"#;
    assert_eq!(parse_pull_request_state(closed).unwrap().mergeable,
               Mergeability::Unknown,
               "a closed pull request cannot be merged whatever the field says");
}

#[test]
fn a_conflicting_pull_request_reads_as_conflicting() {
    for json in [r#"{"state":"OPEN","isDraft":false,"mergeable":"CONFLICTING"}"#,
                 r#"{"state":"OPEN","isDraft":false,"mergeStateStatus":"DIRTY"}"#]
    {
        assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
                   Mergeability::Conflicting,
                   "for {json}");
    }
}

/// A conflict is the first thing to fix, so it outranks everything else
/// wrong with the pull request - here a draft with red CI.
#[test]
fn a_conflict_outranks_every_other_reason() {
    let json = r#"{"state":"OPEN","isDraft":true,"mergeable":"CONFLICTING",
                   "mergeStateStatus":"DIRTY",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"FAILURE"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Conflicting);
}

/// `mergeable` says only that there is no conflict; `mergeStateStatus` is
/// the one field that says the branch needs updating.
#[test]
fn a_branch_behind_its_base_reads_as_behind() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BEHIND"}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Behind);
}

/// Updating the branch reruns the checks, so behind is what the user acts
/// on first.
#[test]
fn behind_outranks_running_checks() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BEHIND",
                   "statusCheckRollup":[{"status":"IN_PROGRESS"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Behind);
}

/// GitHub reports `BLOCKED` while required checks are still pending. That is
/// a wait, not a problem, so it reads as checks running.
#[test]
fn running_checks_read_as_running_even_when_github_says_blocked() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BLOCKED",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"SUCCESS"},
                                        {"status":"QUEUED"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::ChecksRunning);
}

/// Green checks and no conflict, but a required review is outstanding.
#[test]
fn a_pull_request_github_calls_blocked_is_blocked_once_checks_finish() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "mergeStateStatus":"BLOCKED",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"SUCCESS"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Blocked);
}

/// A `gh` too old to send `mergeStateStatus` still colours a mergeable row.
#[test]
fn a_gh_without_merge_state_falls_back_to_mergeable() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE"}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Mergeable);
}

/// GitHub refuses to merge a draft, which is the whole point of marking one.
#[test]
fn a_draft_is_blocked_even_when_github_calls_it_mergeable() {
    let json = r#"{"state":"OPEN","isDraft":true,"mergeable":"MERGEABLE"}"#;
    let state = parse_pull_request_state(json).unwrap();

    assert_eq!(state.status, PullRequestStatus::Draft);
    assert_eq!(state.mergeable, Mergeability::Blocked);
}

/// A pull request with red CI is not one anybody is about to merge, whatever
/// the API says about conflicts.
#[test]
fn failing_checks_block_an_otherwise_mergeable_pull_request() {
    let json = r#"{"state":"OPEN","isDraft":false,"mergeable":"MERGEABLE",
                   "statusCheckRollup":[{"status":"COMPLETED","conclusion":"FAILURE"}]}"#;

    assert_eq!(parse_pull_request_state(json).unwrap().mergeable,
               Mergeability::Blocked);
}

/// A row must not claim a colour it has not earned.
#[test]
fn an_uncomputed_mergeability_stays_unknown() {
    for value in [r#""UNKNOWN""#, "null"] {
        let json = format!(r#"{{"state":"OPEN","isDraft":false,"mergeable":{value}}}"#);

        assert_eq!(parse_pull_request_state(&json).unwrap().mergeable,
                   Mergeability::Unknown,
                   "for {value}");
    }
}

/// GitHub computes mergeability lazily, so the first ask for an open pull
/// request often answers "unknown" and starts the work. One retry turns that
/// into a single visible fetch.
#[test]
fn an_open_pull_request_with_unknown_mergeability_is_asked_twice() {
    let runner = StubRunner::ok(r#"{"state":"OPEN","isDraft":false,"mergeable":"UNKNOWN"}"#);

    let state = pull_request_state_with(&runner, "https://github.com/a/b/pull/1").unwrap();

    assert_eq!(runner.calls().len(), 2);
    assert_eq!(state.mergeable,
               Mergeability::Unknown,
               "still unknown, and still listed");
}

/// Nothing is left to compute once it is merged, so a second ask would be
/// one `gh` run per row for no answer.
#[test]
fn a_merged_pull_request_is_never_asked_twice() {
    let runner = StubRunner::ok(CAPTURED_MERGED);

    pull_request_state_with(&runner, "https://github.com/a/b/pull/1").unwrap();

    assert_eq!(runner.calls().len(), 1);
}

// --- The retention window ---------------------------------------------------

const DAY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

/// A state merged at `merged_at`, or unmerged when it is `None`.
fn merged(status: PullRequestStatus, merged_at: Option<OffsetDateTime>) -> super::PullRequestState {
    super::PullRequestState { number: Some(42),
                              title: Some("Do the thing".to_owned()),
                              status,
                              checks: Some(CheckRollup::Passing),
                              mergeable: Mergeability::Unknown,
                              merged_at }
}

fn at(rfc3339: &str) -> OffsetDateTime {
    OffsetDateTime::parse(rfc3339, &Rfc3339).unwrap()
}

#[test]
fn a_pull_request_merged_past_the_window_is_past_the_window() {
    let state = merged(PullRequestStatus::Merged, Some(at("2026-01-01T00:00:00Z")));

    assert!(state.merged_longer_than(DAY, at("2026-01-02T00:00:01Z").into()));
}

#[test]
fn a_pull_request_merged_within_the_window_is_not() {
    let state = merged(PullRequestStatus::Merged, Some(at("2026-01-01T00:00:00Z")));

    assert!(!state.merged_longer_than(DAY, at("2026-01-01T23:59:59Z").into()));
}

/// The boundary is strict, so "a day old" is still listed and the rule reads
/// as "merged longer than a day ago" rather than "a day or more".
#[test]
fn exactly_the_window_is_not_past_it() {
    let state = merged(PullRequestStatus::Merged, Some(at("2026-01-01T00:00:00Z")));

    assert!(!state.merged_longer_than(DAY, at("2026-01-02T00:00:00Z").into()));
}

/// A `gh` too old to report `mergedAt`. The record never expires, which is the
/// documented degradation rather than a reason to drop it.
#[test]
fn merged_with_no_merge_time_is_never_past_the_window() {
    let state = merged(PullRequestStatus::Merged, None);

    assert!(!state.merged_longer_than(DAY, at("2030-01-01T00:00:00Z").into()));
}

#[test]
fn an_unmerged_pull_request_is_never_past_the_window() {
    for status in [PullRequestStatus::Open,
                   PullRequestStatus::Draft,
                   PullRequestStatus::Closed]
    {
        let state = merged(status, Some(at("2020-01-01T00:00:00Z")));

        assert!(!state.merged_longer_than(DAY, at("2030-01-01T00:00:00Z").into()),
                "{status:?}");
    }
}

/// This machine's clock behind the forge's. The span is negative, which is not
/// a `Duration` - so the record stays rather than being dropped on arithmetic
/// nobody intended.
#[test]
fn a_merge_time_in_the_future_keeps_the_record() {
    let state = merged(PullRequestStatus::Merged, Some(at("2030-01-01T00:00:00Z")));

    assert!(!state.merged_longer_than(DAY, at("2026-01-01T00:00:00Z").into()));
}
