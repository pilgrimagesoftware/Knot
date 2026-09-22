//! Unit tests for [`super`]. No `git` here: what this decides is *when* to
//! ask and *whether the answer changed*, both of which are the reason a
//! dashboard can draw a card without running a subprocess.

use std::time::Duration;

use uuid::Uuid;

use super::{DiffStatsCache, MAX_AGE};

/// Always claims, so a test can record twice without waiting out a TTL.
const ALWAYS: Duration = Duration::ZERO;

fn stats(insertions: u64) -> Option<knot_git::DiffStats> {
    Some(knot_git::DiffStats { insertions,
                               deletions: 0,
                               files_changed: 1 })
}

#[test]
fn a_fresh_entry_is_not_asked_for_again() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    let writer = cache.claim_refresh(id, MAX_AGE)
                      .expect("the first ask claims a refresh");
    writer.record(stats(1));

    assert!(cache.claim_refresh(id, MAX_AGE).is_none(),
            "a stat within MAX_AGE must not spawn a second `git`");
}

/// Claiming marks the request before the answer arrives, so a `git` call
/// slower than a frame is not started once per frame until it returns.
#[test]
fn a_refresh_in_flight_is_not_claimed_twice() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    let _in_flight = cache.claim_refresh(id, MAX_AGE).expect("claimed");

    assert!(cache.claim_refresh(id, MAX_AGE).is_none());
}

#[test]
fn a_recorded_stat_reaches_the_snapshot() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    cache.claim_refresh(id, MAX_AGE)
         .expect("claimed")
         .record(stats(3));

    assert_eq!(cache.snapshot().get(&id), Some(&stats(3)));
}

/// The distinction the render path depends on: a missing entry is "not asked
/// yet", a recorded `None` is "asked, and the folder is not a git checkout" -
/// which must stop the card saying it is still working.
#[test]
fn a_folder_that_is_not_a_repository_records_an_answer() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    assert_eq!(cache.snapshot().get(&id), None);
    cache.claim_refresh(id, MAX_AGE)
         .expect("claimed")
         .record(None);

    assert_eq!(cache.snapshot().get(&id), Some(&None));
}

#[test]
fn the_changed_flag_is_set_once_and_cleared_by_reading_it() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    cache.claim_refresh(id, MAX_AGE)
         .expect("claimed")
         .record(stats(1));

    assert!(cache.take_changed());
    assert!(!cache.take_changed(), "reading the flag clears it");
}

/// Most refreshes find the same numbers. Repainting for those would undo the
/// point of the cache, so only a *different* stat counts as a change.
#[test]
fn an_unchanged_stat_does_not_ask_for_a_repaint() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    cache.claim_refresh(id, ALWAYS)
         .expect("claimed")
         .record(stats(1));
    assert!(cache.take_changed(), "the first answer is new");

    cache.claim_refresh(id, ALWAYS)
         .expect("claimed")
         .record(stats(1));

    assert!(!cache.take_changed(), "the same numbers must not repaint");
}

#[test]
fn a_changed_stat_does_ask_for_a_repaint() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    cache.claim_refresh(id, ALWAYS)
         .expect("claimed")
         .record(stats(1));
    cache.take_changed();

    cache.claim_refresh(id, ALWAYS)
         .expect("claimed")
         .record(stats(2));

    assert!(cache.take_changed());
}

#[test]
fn forgetting_an_agent_drops_its_value_and_its_request_time() {
    let mut cache = DiffStatsCache::default();
    let id = Uuid::new_v4();

    cache.claim_refresh(id, MAX_AGE)
         .expect("claimed")
         .record(stats(1));
    cache.forget(&id);

    assert_eq!(cache.snapshot().get(&id), None);
    assert!(cache.claim_refresh(id, MAX_AGE).is_some(),
            "a forgotten agent is asked for again rather than staying fresh");
}
