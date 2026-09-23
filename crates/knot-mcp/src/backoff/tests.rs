//! Unit tests for [`super`].

use super::*;

#[test]
fn the_first_attempt_waits_the_initial_delay() {
    assert_eq!(backoff_delay(1), BACKOFF_INITIAL_DELAY);
}

#[test]
fn an_attempt_number_below_one_is_treated_as_the_first() {
    assert_eq!(backoff_delay(0), BACKOFF_INITIAL_DELAY);
}

#[test]
fn the_delay_grows_by_the_multiplier_until_it_caps() {
    let mut previous = backoff_delay(1);
    for attempt in 2..=6 {
        let delay = backoff_delay(attempt);
        assert!(delay >= previous,
                "attempt {attempt} shrank: {previous:?} -> {delay:?}");
        if delay < BACKOFF_MAX_DELAY {
            assert_eq!(delay,
                       previous * BACKOFF_MULTIPLIER,
                       "attempt {attempt} did not multiply");
        }
        previous = delay;
    }
}

#[test]
fn the_delay_never_exceeds_the_maximum() {
    for attempt in [1, 2, 8, 32, 1_000, u32::MAX] {
        assert!(backoff_delay(attempt) <= BACKOFF_MAX_DELAY,
                "attempt {attempt} exceeded the cap");
    }
}

#[test]
fn a_large_attempt_count_saturates_at_the_cap_rather_than_wrapping() {
    assert_eq!(backoff_delay(u32::MAX), BACKOFF_MAX_DELAY);
    assert_eq!(backoff_delay(64), BACKOFF_MAX_DELAY);
}
