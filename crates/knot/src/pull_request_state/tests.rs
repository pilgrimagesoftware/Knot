//! Unit tests for [`super`]. No `gh` here: what this decides is when to ask,
//! what a row shows while an answer is in flight, and how the sidebar's
//! breakdown is counted.

use std::time::Duration;

use knot_forge::{
    CheckRollup, ForgeAvailability, Mergeability, PullRequestState, PullRequestStatus,
};

use super::{ForgeStatus, MAX_AGE, PullRequestStateCache, counts_for};

const ALWAYS: Duration = Duration::ZERO;
const URL: &str = "https://github.com/acme/widget/pull/42";

fn state(status: PullRequestStatus) -> Option<PullRequestState> {
    Some(PullRequestState { number: Some(42),
                            title: Some("Do the thing".to_string()),
                            status,
                            checks: Some(CheckRollup::Passing),
                            mergeable: Mergeability::Mergeable })
}

fn urls(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("https://github.com/acme/widget/pull/{n}"))
               .collect()
}

// --- The cache --------------------------------------------------------------

#[test]
fn a_url_with_no_answer_yet_reads_as_absent() {
    let cache = PullRequestStateCache::default();

    assert_eq!(cache.get(&URL.to_string()), None);
}

#[test]
fn a_fetched_state_is_readable_and_is_not_asked_for_again() {
    let mut cache = PullRequestStateCache::default();

    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));

    assert_eq!(cache.get(&URL.to_string()),
               Some(state(PullRequestStatus::Open)));
    assert!(cache.claim_refresh(URL.to_string(), MAX_AGE).is_none(),
            "a state within MAX_AGE must not spawn a second `gh`");
}

/// The distinction the row depends on: absent is "not asked yet", a recorded
/// `None` is "asked, and the fetch failed" - which must stop the row saying
/// it is still loading.
#[test]
fn a_failed_fetch_records_an_answer_rather_than_staying_pending() {
    let mut cache = PullRequestStateCache::default();

    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(None);

    assert_eq!(cache.get(&URL.to_string()), Some(None));
}

/// The spec's "refreshing does not blank the row": the value is replaced
/// when the answer lands, never cleared when the fetch starts.
#[test]
fn a_row_keeps_its_state_while_a_refresh_is_in_flight() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));

    let in_flight = cache.claim_refresh(URL.to_string(), ALWAYS)
                         .expect("claimed");

    assert_eq!(cache.get(&URL.to_string()),
               Some(state(PullRequestStatus::Open)),
               "still showing the state it has");
    in_flight.record(state(PullRequestStatus::Merged));
    assert_eq!(cache.get(&URL.to_string()),
               Some(state(PullRequestStatus::Merged)));
}

/// A pull request nobody has touched comes back the same every minute.
/// Repainting for those would undo the point of the cache.
#[test]
fn an_unchanged_state_does_not_ask_for_a_repaint() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));
    assert!(cache.take_changed());

    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Open));
    assert!(!cache.take_changed());

    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(state(PullRequestStatus::Merged));
    assert!(cache.take_changed(), "a merge must reach the view");
}

// --- The breakdown ----------------------------------------------------------

#[test]
fn nothing_recorded_counts_as_nothing() {
    let counts = counts_for(&PullRequestStateCache::default().snapshot(), &[]);

    assert_eq!(counts.total(), 0);
    assert!(counts.nothing_known());
}

/// The spec's scenario: four records, two open (one of them draft), one
/// merged, one closed.
#[test]
fn the_breakdown_counts_draft_as_open() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(4);
    for (url, status) in urls.iter().zip([PullRequestStatus::Open,
                                          PullRequestStatus::Draft,
                                          PullRequestStatus::Merged,
                                          PullRequestStatus::Closed])
    {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(status));
    }

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!(counts.open, 2, "draft counts as open");
    assert_eq!(counts.merged, 1);
    assert_eq!(counts.closed, 1);
    assert_eq!(counts.pending, 0);
    assert_eq!(counts.total(), 4);
    assert!(!counts.nothing_known());
}

/// Before the view has been shown, no state has been fetched - so the row
/// shows the total rather than a breakdown that would read as "all of these
/// are pending".
#[test]
fn with_no_state_fetched_nothing_is_known_and_the_total_still_counts() {
    let cache = PullRequestStateCache::default();
    let urls = urls(3);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert!(counts.nothing_known());
    assert_eq!(counts.pending, 3);
    assert_eq!(counts.total(), 3);
}

/// The spec's scenario: three have state and one request failed. The failed
/// one is counted as pending rather than folded into a state.
#[test]
fn a_failed_fetch_is_counted_as_pending_not_as_a_state() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(4);
    for (url, status) in urls.iter().zip([PullRequestStatus::Open,
                                          PullRequestStatus::Merged,
                                          PullRequestStatus::Closed])
    {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(status));
    }
    cache.claim_refresh(urls[3].clone(), MAX_AGE)
         .expect("claimed")
         .record(None);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!((counts.open, counts.merged, counts.closed), (1, 1, 1));
    assert_eq!(counts.pending, 1);
    assert!(!counts.nothing_known());
}

/// A state that belongs to another workspace's record must not reach this
/// workspace's row.
#[test]
fn only_the_urls_asked_about_are_counted() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(3);
    for url in &urls {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(PullRequestStatus::Open));
    }

    let counts = counts_for(&cache.snapshot(), &urls[..1]);

    assert_eq!(counts.open, 1);
    assert_eq!(counts.total(), 1);
}

// --- Availability -----------------------------------------------------------

#[test]
fn availability_starts_unprobed() {
    let status = ForgeStatus::default();

    assert!(status.needs_probe());
    assert!(!status.is_ready());
    assert!(status.availability().is_none());
}

#[test]
fn only_a_ready_forge_is_worth_fetching_from() {
    for (availability, ready) in [(ForgeAvailability::Ready, true),
                                  (ForgeAvailability::Missing, false),
                                  (ForgeAvailability::Unauthenticated, false),
                                  (ForgeAvailability::Failed("boom".to_string()), false)]
    {
        let mut status = ForgeStatus::default();
        status.set(availability.clone());

        assert_eq!(status.is_ready(), ready, "for {availability:?}");
        assert!(!status.needs_probe(), "probed once, whatever it found");
        assert_eq!(status.availability(), Some(&availability));
    }
}
