//! The per-URL pull request state cache the Pull Requests view reads.
//!
//! Contract: the `pull-request-tracking` capability spec under
//! `openspec/changes/pull-request-tracking/specs/`.
//!
//! `gh pr view` is a subprocess over the network - slower than `git diff`,
//! and a list of twenty rows would be twenty of them per frame. So nothing
//! fetches state where it is drawn: this is [`crate::refresh_cache`] keyed by
//! canonical URL, and the row draws whatever has landed.
//!
//! State is never persisted. A merged pull request shown as open after a
//! restart is worse than a blank, and there is no way to know a remembered
//! status is still true - so the cache starts empty every launch and the
//! rows fill in as answers arrive.

use std::collections::BTreeMap;
use std::time::Duration;

use knot_forge::{ForgeAvailability, PullRequestState, PullRequestStatus};

use crate::consts;
use crate::refresh_cache::RefreshCache;

/// Last known state per pull request URL, and when each was last requested.
///
/// The value's `Option` is the *lookup*: `None` means the fetch finished and
/// failed, which is a finished answer and not a pending one. A URL absent
/// from the cache is the pending case - nothing has been asked for it yet.
pub(crate) type PullRequestStateCache = RefreshCache<String, Option<PullRequestState>>;

/// How stale a pull request's state may be before it is fetched again.
pub(crate) const MAX_AGE: Duration = consts::PULL_REQUEST_STATE_MAX_AGE;

/// How many of a workspace's recorded pull requests are in each state.
///
/// What the sidebar's launcher row shows. Draft counts as open, matching the
/// forge's own model in which draft is a property of an open pull request
/// rather than a fourth state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PullRequestCounts {
    pub(crate) open:    usize,
    pub(crate) merged:  usize,
    pub(crate) closed:  usize,
    /// Recorded, but with no state fetched: either nothing has been asked
    /// for it yet, or the fetch failed. Counted separately rather than
    /// folded into a state, because the row must not imply a state Knot has
    /// not fetched.
    pub(crate) pending: usize,
}

impl PullRequestCounts {
    /// How many records there are in total.
    pub(crate) fn total(self) -> usize {
        self.open + self.merged + self.closed + self.pending
    }

    /// Whether nothing at all is known yet, so the row shows the total
    /// instead of a breakdown.
    ///
    /// True before the view has been shown for the first time, and on a
    /// machine where `gh` is unavailable - in both cases a breakdown would
    /// read as "all four of these are pending", which says less than "four".
    pub(crate) fn nothing_known(self) -> bool {
        self.open == 0 && self.merged == 0 && self.closed == 0
    }
}

/// Count `urls` by the state the cache holds for each.
///
/// Takes the URLs rather than the records so the caller decides the scope -
/// one workspace's, in every current use.
pub(crate) fn counts_for(cache: &BTreeMap<String, Option<PullRequestState>>, urls: &[String])
                         -> PullRequestCounts {
    let mut counts = PullRequestCounts::default();
    for url in urls {
        match cache.get(url).and_then(Option::as_ref) {
            Some(state) if state.status == PullRequestStatus::Merged => counts.merged += 1,
            Some(state) if state.status == PullRequestStatus::Closed => counts.closed += 1,
            // Draft and open both; see `PullRequestCounts`.
            Some(_) => counts.open += 1,
            None => counts.pending += 1,
        }
    }
    counts
}

/// Whether state can be fetched at all right now.
///
/// Probed once per view opening rather than once per row, and held so the
/// view's single availability message has something to read. `None` while it
/// has never been probed, which is every moment before the view is first
/// shown.
#[derive(Debug, Clone, Default)]
pub(crate) struct ForgeStatus {
    availability: Option<ForgeAvailability>,
}

impl ForgeStatus {
    /// What the last probe found, or `None` if there has not been one.
    pub(crate) fn availability(&self) -> Option<&ForgeAvailability> {
        self.availability.as_ref()
    }

    /// Record what a probe found.
    pub(crate) fn set(&mut self, availability: ForgeAvailability) {
        self.availability = Some(availability);
    }

    /// Whether state is worth fetching. False before the first probe, so a
    /// view that has just opened asks `gh` what it is dealing with before it
    /// asks about twenty pull requests.
    pub(crate) fn is_ready(&self) -> bool {
        self.availability
            .as_ref()
            .is_some_and(ForgeAvailability::is_ready)
    }

    /// Whether a probe is still owed.
    pub(crate) fn needs_probe(&self) -> bool {
        self.availability.is_none()
    }
}

#[cfg(test)]
mod tests;
