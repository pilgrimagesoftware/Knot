//! Unit tests for [`super`], against a value type neither user has, so what
//! is under test is the mechanism rather than either caller's semantics.
//!
//! The diff-stat and pull-request caches each test their own naming and
//! cadence on top of this.

use std::time::Duration;

use super::RefreshCache;

const ALWAYS: Duration = Duration::ZERO;
const NEVER: Duration = Duration::from_secs(3600);

fn cache() -> RefreshCache<String, u32> {
    RefreshCache::default()
}

#[test]
fn a_key_with_no_answer_yet_reads_as_absent() {
    let cache = cache();

    assert_eq!(cache.get(&"a".to_string()), None);
    assert!(cache.snapshot().is_empty());
}

#[test]
fn a_recorded_value_is_readable() {
    let mut cache = cache();

    cache.claim_refresh("a".to_string(), NEVER)
         .expect("claimed")
         .record(7);

    assert_eq!(cache.get(&"a".to_string()), Some(7));
}

/// Claiming marks the request before the answer arrives, so work slower than
/// a frame is not started once per frame until it returns.
#[test]
fn a_claim_in_flight_blocks_a_second_claim() {
    let mut cache = cache();

    let _in_flight = cache.claim_refresh("a".to_string(), NEVER)
                          .expect("claimed");

    assert!(cache.claim_refresh("a".to_string(), NEVER).is_none());
}

#[test]
fn a_value_older_than_the_max_age_is_claimed_again() {
    let mut cache = cache();
    cache.claim_refresh("a".to_string(), ALWAYS)
         .expect("claimed")
         .record(7);

    assert!(cache.claim_refresh("a".to_string(), ALWAYS).is_some());
}

/// The max age is the caller's, not the cache's: two callers of one cache
/// may want different cadences, and the same key asked under a longer age is
/// still fresh.
#[test]
fn freshness_is_judged_against_the_age_the_caller_passes() {
    let mut cache = cache();
    cache.claim_refresh("a".to_string(), NEVER)
         .expect("claimed")
         .record(7);

    assert!(cache.claim_refresh("a".to_string(), NEVER).is_none());
    assert!(cache.claim_refresh("a".to_string(), ALWAYS).is_some());
}

#[test]
fn only_a_different_value_flags_a_change() {
    let mut cache = cache();

    cache.claim_refresh("a".to_string(), ALWAYS)
         .expect("claimed")
         .record(7);
    assert!(cache.take_changed(), "the first answer is new");
    assert!(!cache.take_changed(), "reading the flag clears it");

    cache.claim_refresh("a".to_string(), ALWAYS)
         .expect("claimed")
         .record(7);
    assert!(!cache.take_changed(), "the same value must not repaint");

    cache.claim_refresh("a".to_string(), ALWAYS)
         .expect("claimed")
         .record(8);
    assert!(cache.take_changed());
}

/// The reason a refresh does not blank its row: the value is replaced when
/// the answer lands, never cleared when the work starts.
#[test]
fn a_claim_leaves_the_previous_value_in_place_until_it_is_recorded() {
    let mut cache = cache();
    cache.claim_refresh("a".to_string(), ALWAYS)
         .expect("claimed")
         .record(7);

    let in_flight = cache.claim_refresh("a".to_string(), ALWAYS)
                         .expect("claimed");
    assert_eq!(cache.get(&"a".to_string()),
               Some(7),
               "still showing the old value");

    in_flight.record(8);
    assert_eq!(cache.get(&"a".to_string()), Some(8));
}

/// A writer outliving its window must cost nothing: it holds the shared maps
/// and no reference back.
#[test]
fn a_writer_recording_after_its_cache_is_dropped_does_not_panic() {
    let mut cache = cache();
    let writer = cache.claim_refresh("a".to_string(), NEVER)
                      .expect("claimed");

    drop(cache);

    writer.record(7);
}

#[test]
fn forgetting_a_key_drops_its_value_and_its_request_time() {
    let mut cache = cache();
    cache.claim_refresh("a".to_string(), NEVER)
         .expect("claimed")
         .record(7);

    cache.forget(&"a".to_string());

    assert_eq!(cache.get(&"a".to_string()), None);
    assert!(cache.claim_refresh("a".to_string(), NEVER).is_some(),
            "a forgotten key is asked for again rather than staying fresh");
}

/// Without this, every key a window has ever shown keeps an entry for the
/// window's whole life - including ones that no longer exist.
#[test]
fn retain_drops_every_key_the_predicate_rejects() {
    let mut cache = cache();
    for key in ["a", "b", "c"] {
        cache.claim_refresh(key.to_string(), NEVER)
             .expect("claimed")
             .record(1);
    }

    cache.retain(|key| key == "b");

    assert_eq!(cache.snapshot().keys().cloned().collect::<Vec<_>>(),
               vec!["b".to_string()]);
    assert!(cache.claim_refresh("a".to_string(), NEVER).is_some(),
            "a dropped key's request time went with it");
}

/// `holds` answers from what has landed, and a key with nothing recorded
/// holds nothing - not even for a predicate that accepts everything.
#[test]
fn holds_tests_the_recorded_value_and_only_that() {
    let mut cache = cache();
    cache.claim_refresh("a".to_string(), NEVER)
         .expect("claimed")
         .record(7);

    assert!(cache.holds(&"a".to_string(), |value| *value == 7));
    assert!(!cache.holds(&"a".to_string(), |value| *value == 8));
    assert!(!cache.holds(&"b".to_string(), |_| true));
}

/// Refresh now: every key claims again at once, and the value it will
/// replace is still there to draw until the new one lands.
#[test]
fn marking_stale_reclaims_every_key_and_keeps_its_value() {
    let mut cache = cache();
    cache.claim_refresh("a".to_string(), NEVER)
         .expect("claimed")
         .record(7);
    assert!(cache.claim_refresh("a".to_string(), NEVER).is_none());

    cache.mark_all_stale();

    assert_eq!(cache.get(&"a".to_string()), Some(7));
    assert!(cache.claim_refresh("a".to_string(), NEVER).is_some());
}
