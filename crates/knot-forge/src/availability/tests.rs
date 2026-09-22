//! Unit tests for [`super`].

use super::{ForgeAvailability, probe_with};
use crate::runner::stub::StubRunner;

#[test]
fn an_authenticated_gh_is_ready() {
    let runner = StubRunner::ok("github.com\n  ✓ Logged in to github.com account octocat");

    assert_eq!(probe_with(&runner), ForgeAvailability::Ready);
}

#[test]
fn an_absent_binary_is_missing() {
    assert_eq!(probe_with(&StubRunner::missing()),
               ForgeAvailability::Missing);
}

/// The wording has changed between `gh` versions, so each spelling that has
/// meant "logged out" has to keep meaning it.
#[test]
fn every_logged_out_wording_is_unauthenticated() {
    for output in ["You are not logged into any GitHub hosts. To log in, run: gh auth login",
                   "no accounts satisfied these criteria",
                   "authentication failed for github.com"]
    {
        assert_eq!(probe_with(&StubRunner::failing(output, 1)),
                   ForgeAvailability::Unauthenticated,
                   "for {output:?}");
    }
}

/// A failure that is not "logged out" must not be reported as one: it would
/// send the user to `gh auth login` for a problem that is not there.
#[test]
fn another_failure_is_reported_as_failed_with_its_reason() {
    let runner = StubRunner::failing("dial tcp: lookup github.com: no such host", 1);

    match probe_with(&runner) {
        ForgeAvailability::Failed(reason) => assert!(reason.contains("no such host"), "{reason}"),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn only_ready_is_ready() {
    assert!(ForgeAvailability::Ready.is_ready());
    assert!(!ForgeAvailability::Missing.is_ready());
    assert!(!ForgeAvailability::Unauthenticated.is_ready());
    assert!(!ForgeAvailability::Failed("boom".to_owned()).is_ready());
}

#[test]
fn the_probe_asks_gh_once() {
    let runner = StubRunner::ok("logged in");

    probe_with(&runner);

    assert_eq!(runner.calls(), vec!["auth status"]);
}
