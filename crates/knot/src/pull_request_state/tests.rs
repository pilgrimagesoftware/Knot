//! Unit tests for [`super`]. No `gh` here: what this decides is when to ask,
//! what a row shows while an answer is in flight, and how the sidebar's
//! breakdown is counted.

use std::time::{Duration, SystemTime};

use knot_forge::{
    CheckRollup, ForgeAvailability, ForgeError, Mergeability, PullRequestState, PullRequestStatus,
};

use super::{
    ForgeStatus, MAX_AGE, PullRequestLookup, PullRequestStateCache, counts_for, expired_urls,
};

const ALWAYS: Duration = Duration::ZERO;
const URL: &str = "https://github.com/acme/widget/pull/42";

fn state(status: PullRequestStatus) -> PullRequestLookup {
    PullRequestLookup::Known(PullRequestState { number: Some(42),
                                                title: Some("Do the thing".to_string()),
                                                status,
                                                checks: Some(CheckRollup::Passing),
                                                mergeable: Mergeability::Mergeable,
                                                merged_at: None })
}

/// A merged state carrying a merge time, as a fetch of a merged pull request
/// produces. `into()` rather than a named `OffsetDateTime` so these tests need
/// no date library of their own.
fn merged_at(when: SystemTime) -> PullRequestLookup {
    let PullRequestLookup::Known(merged) = state(PullRequestStatus::Merged)
    else {
        unreachable!("state() builds a known lookup");
    };
    PullRequestLookup::Known(PullRequestState { merged_at: Some(when.into()),
                                                ..merged })
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
/// `Failed` is "asked, and the fetch failed" - a finished answer, not a
/// missing one.
#[test]
fn a_failed_fetch_records_an_answer_rather_than_staying_pending() {
    let mut cache = PullRequestStateCache::default();

    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::Failed);

    assert_eq!(cache.get(&URL.to_string()), Some(PullRequestLookup::Failed));
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
         .record(PullRequestLookup::Failed);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!((counts.open, counts.merged, counts.closed), (1, 1, 1));
    assert_eq!(counts.pending, 1);
    assert!(!counts.nothing_known());
}

/// The forge said one of four does not exist: counted on its own, and neither
/// as a state nor as pending.
#[test]
fn a_pull_request_that_does_not_exist_is_counted_apart() {
    let mut cache = PullRequestStateCache::default();
    let urls = urls(4);
    for url in &urls[..3] {
        cache.claim_refresh(url.clone(), MAX_AGE)
             .expect("claimed")
             .record(state(PullRequestStatus::Open));
    }
    cache.claim_refresh(urls[3].clone(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::NotFound);

    let counts = counts_for(&cache.snapshot(), &urls);

    assert_eq!((counts.open, counts.not_found, counts.pending), (3, 1, 0));
    assert_eq!(counts.total(), 4);
}

/// Not found is an answer: a workspace whose only records are missing pull
/// requests has a breakdown to show, not just a total.
#[test]
fn not_found_alone_counts_as_known() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(PullRequestLookup::NotFound);

    let counts = counts_for(&cache.snapshot(), &[URL.to_string()]);

    assert!(!counts.nothing_known());
}

/// A pull request that does not exist has no merge time, so it never expires:
/// only the user removes it.
#[test]
fn a_record_the_forge_cannot_find_is_kept() {
    let cache = cache_of(PullRequestLookup::NotFound);

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty());
}

// --- The lookup -------------------------------------------------------------

/// The forge's answer, sorted into the three the row distinguishes.
#[test]
fn a_forge_result_becomes_the_matching_lookup() {
    let known = state(PullRequestStatus::Open);
    let PullRequestLookup::Known(open) = known.clone()
    else {
        unreachable!("state() builds a known lookup");
    };

    assert_eq!(PullRequestLookup::from(Ok(open)), known);
    assert_eq!(PullRequestLookup::from(Err(ForgeError::NotFound("gone".to_string()))),
               PullRequestLookup::NotFound);
    assert_eq!(PullRequestLookup::from(Err(ForgeError::Timeout { command: "pr view".to_string(), })),
               PullRequestLookup::Failed);
    assert_eq!(PullRequestLookup::from(Err(ForgeError::Parse("bad".to_string()))),
               PullRequestLookup::Failed);
}

/// Only not found stops the refreshes. A state can change and a failure can
/// clear, so both are asked about again.
#[test]
fn only_not_found_is_final() {
    assert!(PullRequestLookup::NotFound.is_final());
    assert!(!PullRequestLookup::Failed.is_final());
    assert!(!state(PullRequestStatus::Open).is_final());
    assert!(!state(PullRequestStatus::Merged).is_final());
}

/// What the refresh loop asks before claiming: a final answer is held, so no
/// `gh` is spawned for it however long the window stays open.
#[test]
fn a_not_found_answer_is_held_as_final_by_the_cache() {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(PullRequestLookup::NotFound);

    assert!(cache.holds(&URL.to_string(), PullRequestLookup::is_final));
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

    assert!(!status.is_ready());
    assert!(status.availability().is_none());
    assert!(!status.take_changed(), "nothing has landed to draw");
}

#[test]
fn only_a_ready_forge_is_worth_fetching_from() {
    for (availability, ready) in [(ForgeAvailability::Ready, true),
                                  (ForgeAvailability::Missing, false),
                                  (ForgeAvailability::Unauthenticated, false),
                                  (ForgeAvailability::Failed("boom".to_string()), false)]
    {
        let mut status = ForgeStatus::default();
        status.claim_probe(MAX_AGE)
              .expect("the first probe is always claimable")
              .record(availability.clone());

        assert_eq!(status.is_ready(), ready, "for {availability:?}");
        assert_eq!(status.availability(), Some(availability));
    }
}

/// The defect this shape exists for: the probe is `gh auth status`, and the
/// only caller is a render.
#[test]
fn a_probe_is_claimed_once_and_not_again_until_it_ages_out() {
    let mut status = ForgeStatus::default();

    let first = status.claim_probe(MAX_AGE);
    assert!(first.is_some(), "the first frame claims it");
    assert!(status.claim_probe(MAX_AGE).is_none(),
            "every later frame must not, or an open view spawns a subprocess per frame");

    first.expect("claimed").record(ForgeAvailability::Ready);
    assert!(status.claim_probe(MAX_AGE).is_none(),
            "and a landed answer does not re-open the claim within its age");
    assert!(status.claim_probe(Duration::ZERO).is_some(),
            "only ageing out does - which is how signing in takes effect");
}

/// Without this the answer lands in a background task and nothing redraws:
/// on a workspace with nothing else running, the view keeps whatever it had.
#[test]
fn a_landed_probe_marks_the_view_for_repaint() {
    let mut status = ForgeStatus::default();
    let writer = status.claim_probe(MAX_AGE).expect("claimed");

    assert!(!status.take_changed(), "nothing has landed yet");

    writer.record(ForgeAvailability::Unauthenticated);

    assert!(status.take_changed(), "the answer has to reach a frame");
    assert!(!status.take_changed(),
            "and the flag clears, so one answer is one repaint");
}

/// An unauthenticated forge is not a permanent verdict: the user goes and
/// runs `gh auth login`, and the open view has to notice.
#[test]
fn a_later_probe_replaces_an_earlier_answer() {
    let mut status = ForgeStatus::default();
    status.claim_probe(MAX_AGE)
          .expect("claimed")
          .record(ForgeAvailability::Unauthenticated);
    assert!(!status.is_ready());

    status.claim_probe(Duration::ZERO)
          .expect("aged out")
          .record(ForgeAvailability::Ready);

    assert!(status.is_ready(),
            "signing in has to take effect without reopening the window");
}

// --- Expiry -----------------------------------------------------------------

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// A cache holding one answer for `URL`.
fn cache_of(answer: PullRequestLookup) -> std::collections::BTreeMap<String, PullRequestLookup> {
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), MAX_AGE)
         .expect("claimed")
         .record(answer);
    cache.snapshot()
}

#[test]
fn a_record_merged_past_the_window_expires() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - DAY - Duration::from_secs(1)));

    assert_eq!(expired_urls(&cache, &[URL.to_string()], DAY, now),
               vec![URL.to_string()]);
}

#[test]
fn a_record_merged_within_the_window_is_kept() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - Duration::from_secs(60)));

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, now).is_empty());
}

/// Nothing has been asked for it yet. There is no evidence to act on, so the
/// row stays and a later refresh reconsiders it.
#[test]
fn a_record_with_no_answer_yet_is_kept() {
    let cache = PullRequestStateCache::default().snapshot();

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty());
}

/// The fetch finished and failed - a finished answer, but not one that says
/// anything about merging. Dropping on it would lose a record over a network
/// blip.
#[test]
fn a_record_whose_fetch_failed_is_kept() {
    let cache = cache_of(PullRequestLookup::Failed);

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty());
}

#[test]
fn an_unmerged_record_is_kept_however_old() {
    for status in [PullRequestStatus::Open,
                   PullRequestStatus::Draft,
                   PullRequestStatus::Closed]
    {
        let cache = cache_of(state(status));

        assert!(expired_urls(&cache, &[URL.to_string()], DAY, SystemTime::now()).is_empty(),
                "{status:?}");
    }
}

/// Only the URLs the caller named are considered, so one workspace's sweep
/// cannot drop another's records.
#[test]
fn a_cached_expiry_not_in_the_named_urls_is_not_returned() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - DAY - Duration::from_secs(1)));

    let other = "https://github.com/acme/widget/pull/99".to_string();
    assert!(expired_urls(&cache, &[other], DAY, now).is_empty());
}

/// The common frame: nothing expired, so the caller does not write a file.
#[test]
fn nothing_expires_when_every_record_is_fresh() {
    let now = SystemTime::now();
    let cache = cache_of(merged_at(now - Duration::from_secs(1)));

    assert!(expired_urls(&cache, &[URL.to_string()], DAY, now).is_empty());
}

/// Why the repaint chain needs an entry of its own for expiry.
///
/// A merged pull request's answer stops changing once it has merged, and
/// `record` flags the cache as changed only when the value differs. So the
/// 60-second re-fetches that keep happening while the view is open schedule no
/// frame, and expiry - which runs on the render path - would have nothing to
/// run in. `WorkspaceWindow::pull_requests_expiring` is what covers that; this
/// pins the premise it rests on.
#[test]
fn re_recording_an_unchanged_merged_state_does_not_flag_the_cache() {
    let merged = merged_at(SystemTime::now() - DAY * 2);
    let mut cache = PullRequestStateCache::default();
    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(merged.clone());
    assert!(cache.take_changed(), "the first answer is a change");

    cache.claim_refresh(URL.to_string(), ALWAYS)
         .expect("claimed")
         .record(merged);

    assert!(!cache.take_changed(),
            "an identical answer schedules no frame - so expiry needs its own chain entry");
}
