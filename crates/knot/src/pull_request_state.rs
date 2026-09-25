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
//!
//! That is also why expiry is decided here rather than from the record. A
//! record knows when Knot first saw its URL, which says nothing about when the
//! pull request merged; the merge time arrives with the fetched state, so
//! [`expired_urls`] reads the cache and the retention window together. After a
//! restart nothing expires until the answers land again, which is the same
//! rule as every other thing this cache decides.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use knot_forge::{ForgeAvailability, ForgeError, PullRequestState};
use parking_lot::Mutex;

use crate::consts;
use crate::pull_request_filter::RowCategory;
use crate::refresh_cache::{RefreshCache, RefreshWriter};

/// Last known state per pull request URL, and when each was last requested.
///
/// A URL absent from the cache is the pending case - nothing has been asked
/// for it yet. Every value is a finished answer; see [`PullRequestLookup`].
pub(crate) type PullRequestStateCache = RefreshCache<String, PullRequestLookup>;

/// What one fetch of a pull request's state came back with.
///
/// Three finished answers, kept apart because each means something different
/// to the row: a state to show; the forge saying there is no such pull
/// request; or a fetch that failed and will be tried again. Collapsing the
/// last two is how a pull request that does not exist sat on "Checking…"
/// forever, re-fetched every cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PullRequestLookup {
    Known(PullRequestState),
    /// The forge has no such pull request - or none this `gh` identity can
    /// read, which it answers the same way. Shown, never acted on.
    NotFound,
    /// The fetch did not produce an answer: a timeout, a network error, a
    /// response that would not parse.
    Failed,
}

impl PullRequestLookup {
    /// The state, when there is one.
    pub(crate) fn state(&self) -> Option<&PullRequestState> {
        match self {
            Self::Known(state) => Some(state),
            Self::NotFound | Self::Failed => None,
        }
    }

    /// Whether asking again in this window would be wasted.
    ///
    /// Only not found is final. A state can change and a failure can clear;
    /// a pull request that does not exist will not start existing, and the
    /// one case where it seems to - a repository readable once `gh` is signed
    /// in to the right account - is recovered by a relaunch, since nothing
    /// here is persisted.
    pub(crate) fn is_final(&self) -> bool {
        matches!(self, Self::NotFound)
    }
}

impl From<knot_forge::Result<PullRequestState>> for PullRequestLookup {
    fn from(result: knot_forge::Result<PullRequestState>) -> Self {
        match result {
            Ok(state) => Self::Known(state),
            Err(ForgeError::NotFound(_)) => Self::NotFound,
            Err(_) => Self::Failed,
        }
    }
}

/// How stale a pull request's state may be before it is fetched again.
pub(crate) const MAX_AGE: Duration = consts::PULL_REQUEST_STATE_MAX_AGE;

/// How stale the forge availability answer may be before it is asked again.
pub(crate) const PROBE_MAX_AGE: Duration = consts::FORGE_PROBE_MAX_AGE;

/// How long a merged pull request stays listed after it merged.
pub(crate) const MERGED_RETENTION: Duration = consts::PULL_REQUEST_MERGED_RETENTION;

/// Which of `urls` have been merged long enough to stop listing.
///
/// Every case where the evidence is missing keeps the record, and there are
/// four of them: the URL is not in the cache (nothing asked yet), its entry
/// has no state (the fetch failed, or the forge found no such pull request),
/// it is merged with no merge time the forge reported, or it is not merged at
/// all. Knot drops what it has observed
/// and declines to guess at the rest - a wrongly kept record is a row the user
/// can remove, a wrongly dropped one is work that silently vanished.
///
/// Pure, and takes `now` rather than reading the clock, so the window's edges
/// are testable without waiting a day at either end. The comparison itself
/// lives on [`PullRequestState`], where the merge time does.
pub(crate) fn expired_urls(cache: &BTreeMap<String, PullRequestLookup>, urls: &[String],
                           retention: Duration, now: SystemTime)
                           -> Vec<String> {
    urls.iter()
        .filter(|url| {
            cache.get(*url)
                 .and_then(PullRequestLookup::state)
                 .is_some_and(|state| state.merged_longer_than(retention, now))
        })
        .cloned()
        .collect()
}

/// How many of a workspace's recorded pull requests are in each state.
///
/// What the sidebar's launcher row shows. Draft counts as open, matching the
/// forge's own model in which draft is a property of an open pull request
/// rather than a fourth state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PullRequestCounts {
    pub(crate) open:      usize,
    pub(crate) merged:    usize,
    pub(crate) closed:    usize,
    /// Recorded, but with no state fetched: either nothing has been asked
    /// for it yet, or the fetch failed. Counted separately rather than
    /// folded into a state, because the row must not imply a state Knot has
    /// not fetched.
    pub(crate) pending:   usize,
    /// Recorded, and the forge says it does not exist. Its own count rather
    /// than pending, since the answer is in, and rather than any state, since
    /// there is none.
    pub(crate) not_found: usize,
}

impl PullRequestCounts {
    /// How many records there are in total.
    pub(crate) fn total(self) -> usize {
        self.open + self.merged + self.closed + self.pending + self.not_found
    }

    /// Whether nothing at all is known yet, so the row shows the total
    /// instead of a breakdown.
    ///
    /// True before the view has been shown for the first time, and on a
    /// machine where `gh` is unavailable - in both cases a breakdown would
    /// read as "all four of these are pending", which says less than "four".
    /// Not found counts as known: it is an answer, and "2 not found" says
    /// more than "2 recorded".
    pub(crate) fn nothing_known(self) -> bool {
        self.open == 0 && self.merged == 0 && self.closed == 0 && self.not_found == 0
    }
}

/// Count `urls` by the state the cache holds for each.
///
/// Takes the URLs rather than the records so the caller decides the scope -
/// one workspace's, in every current use.
pub(crate) fn counts_for(cache: &BTreeMap<String, PullRequestLookup>, urls: &[String])
                         -> PullRequestCounts {
    let mut counts = PullRequestCounts::default();
    for url in urls {
        // The one classification the view's status toggles also read, so the
        // sidebar and the toggles cannot disagree; see `RowCategory`.
        match RowCategory::of(cache.get(url)) {
            RowCategory::Open => counts.open += 1,
            RowCategory::Merged => counts.merged += 1,
            RowCategory::Closed => counts.closed += 1,
            RowCategory::NotFound => counts.not_found += 1,
            RowCategory::Pending => counts.pending += 1,
        }
    }
    counts
}

/// Claim a refresh for each of `urls` that is due, returning the URL and the
/// writer for each one claimed.
///
/// A URL whose last answer was final (not found) is skipped whatever its age,
/// unless it is in `asked_again` - the set Refresh now fills. Claiming one
/// takes it out of that set, so the user's refresh asks once more and later
/// cycles go back to skipping it.
///
/// Pure over the cache, so the rule is testable without a window; the caller
/// hands each writer to `spawn_blocking`.
pub(crate) fn claim_refreshes(cache: &mut PullRequestStateCache, urls: &[String],
                              asked_again: &mut BTreeSet<String>, max_age: Duration)
                              -> Vec<(String, RefreshWriter<String, PullRequestLookup>)> {
    let mut claimed = Vec::new();
    for url in urls {
        if !asked_again.contains(url) && cache.holds(url, PullRequestLookup::is_final) {
            continue;
        }
        let Some(writer) = cache.claim_refresh(url.clone(), max_age)
        else {
            continue;
        };
        asked_again.remove(url);
        claimed.push((url.clone(), writer));
    }
    claimed
}

/// Whether state can be fetched at all right now.
///
/// Probed once per view opening rather than once per row, and held so the
/// view's single availability message has something to read. `None` while it
/// has never been probed, which is every moment before the view is first
/// shown.
///
/// The same claim/writer shape as [`crate::refresh_cache`], and for the same
/// reason: a probe is `gh auth status`, a subprocess, and the only place
/// that wants the answer is a render. Claiming marks the request
/// immediately, the work runs off the main thread, and a later frame draws
/// what landed.
#[derive(Debug, Default)]
pub(crate) struct ForgeStatus {
    availability: Arc<Mutex<Option<ForgeAvailability>>>,
    /// Main-thread only: when a probe was last *requested*, so it runs on a
    /// cadence rather than once per render.
    requested:    Option<Instant>,
    dirty:        Arc<AtomicBool>,
}

impl ForgeStatus {
    /// What the last probe found, or `None` if there has not been one.
    pub(crate) fn availability(&self) -> Option<ForgeAvailability> {
        self.availability.lock().clone()
    }

    /// Claims a probe if none has been requested within `max_age`.
    ///
    /// Returns the writer the caller hands to whatever runs the subprocess,
    /// or `None` when the last answer is still fresh - which is the common
    /// case, since this is asked once per frame while the view is open.
    ///
    /// Ageing out rather than probing once per window is what lets the view
    /// recover: a user who was signed out, ran `gh auth login` and came back
    /// would otherwise keep reading "not authenticated" until the window was
    /// closed and reopened.
    pub(crate) fn claim_probe(&mut self, max_age: Duration) -> Option<ForgeProbeWriter> {
        if self.requested.is_some_and(|at| at.elapsed() < max_age) {
            return None;
        }
        self.requested = Some(Instant::now());
        Some(ForgeProbeWriter { availability: Arc::clone(&self.availability),
                                dirty:        Arc::clone(&self.dirty), })
    }

    /// Forget when the last probe was requested, so the next claim runs one
    /// whatever its age. Keeps the last answer, so the view's message does
    /// not blank while the new probe runs. For Refresh now.
    pub(crate) fn mark_stale(&mut self) {
        self.requested = None;
    }

    /// Whether state is worth fetching. False before the first probe lands,
    /// so a view that has just opened asks `gh` what it is dealing with
    /// before it asks about twenty pull requests.
    pub(crate) fn is_ready(&self) -> bool {
        self.availability
            .lock()
            .as_ref()
            .is_some_and(ForgeAvailability::is_ready)
    }

    /// Whether a probe has landed since this was last asked, clearing the
    /// flag. For the poll that decides whether to redraw.
    pub(crate) fn take_changed(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }
}

/// Records one probe's answer from whatever thread ran it.
pub(crate) struct ForgeProbeWriter {
    availability: Arc<Mutex<Option<ForgeAvailability>>>,
    dirty:        Arc<AtomicBool>,
}

impl ForgeProbeWriter {
    /// Stores what the probe found and marks the view for a repaint.
    pub(crate) fn record(self, availability: ForgeAvailability) {
        *self.availability.lock() = Some(availability);
        self.dirty.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests;
