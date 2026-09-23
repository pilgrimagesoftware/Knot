//! The supervisor's retry schedule.
//!
//! Implements the backoff half of the start-attempt requirement in
//! `openspec/changes/supervise-mcp-server/specs/mcp-server/spec.md`.
//!
//! A pure function of the attempt number, so the schedule is verifiable
//! without a server, a clock or a runtime. The supervisor holds the attempt
//! count; this holds no state of its own.

use std::time::Duration;

use crate::consts::{BACKOFF_INITIAL_DELAY, BACKOFF_MAX_DELAY, BACKOFF_MULTIPLIER};

/// The wait after the `attempt`-th failed attempt, counting from one, on
/// the schedule the constants describe.
///
/// Grows from [`BACKOFF_INITIAL_DELAY`] by [`BACKOFF_MULTIPLIER`] per
/// attempt and stops at [`BACKOFF_MAX_DELAY`].
pub fn backoff_delay(attempt: u32) -> Duration {
    delay_for(attempt,
              BACKOFF_INITIAL_DELAY,
              BACKOFF_MULTIPLIER,
              BACKOFF_MAX_DELAY)
}

/// The same schedule against parameters given rather than the constants, so
/// a supervisor tuned for a test runs the identical policy on a shorter
/// clock instead of a second implementation of it.
///
/// The multiplication is checked, so a large attempt count saturates at the
/// cap rather than overflowing into a short delay.
pub(crate) fn delay_for(attempt: u32, initial: Duration, multiplier: u32, max: Duration)
                        -> Duration {
    let steps = attempt.saturating_sub(1);
    let Some(factor) = multiplier.checked_pow(steps)
    else {
        return max;
    };
    initial.saturating_mul(factor).min(max)
}

#[cfg(test)]
mod tests;
