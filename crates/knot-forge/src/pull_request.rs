//! Reading one pull request's current state.

use std::time::{Duration, SystemTime};

use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::consts::{MERGEABILITY_RETRY_DELAY, PULL_REQUEST_FIELDS};
use crate::error::{ForgeError, Result};
use crate::runner::{ForgeRunner, GhRunner};

/// Where a pull request stands.
///
/// Draft is a variant here rather than a flag beside `Open` because it is what
/// the row shows, and a row cannot be two things at once. GitHub models it the
/// other way round - `state: OPEN` plus `isDraft` - so the flattening happens
/// at the parse, in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestStatus {
    Draft,
    Open,
    Merged,
    Closed,
}

impl PullRequestStatus {
    /// Whether this pull request is still awaiting a decision. Draft counts,
    /// which is what makes the launcher row's breakdown three numbers rather
    /// than four.
    #[must_use]
    pub fn is_open(self) -> bool {
        matches!(self, Self::Draft | Self::Open)
    }
}

/// Whether an open pull request can actually be merged.
///
/// Separate from [`PullRequestStatus`] because it is a different question:
/// "is it still open" versus "could it land right now". Only meaningful
/// while a pull request is open - a merged one has nothing left to block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mergeability {
    /// Nothing in the way.
    Mergeable,
    /// Something is: a conflict, a failing check, a branch behind its base,
    /// a review still required, or the pull request being a draft. The
    /// distinctions matter on GitHub's own page; here they collapse, because
    /// a row has one colour and they all mean "not yet".
    Blocked,
    /// GitHub has not computed it. It does so lazily, so this is the honest
    /// answer for a moment after the first ask - and a row must not claim a
    /// colour it has not earned.
    Unknown,
}

/// The summary result of a pull request's checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckRollup {
    Passing,
    Failing,
    Pending,
}

/// What a row shows, fetched fresh each time.
///
/// Every field but the status is optional: a `gh` too old to report one of
/// them degrades that part of the row rather than failing the record. A
/// pull request with no checks configured has no rollup, which is not the
/// same as one whose checks have not finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestState {
    pub number:    Option<u64>,
    pub title:     Option<String>,
    pub status:    PullRequestStatus,
    pub checks:    Option<CheckRollup>,
    /// Whether it could land right now. [`Mergeability::Unknown`] for a
    /// pull request that is not open, where the question does not arise.
    pub mergeable: Mergeability,
    /// When it merged, for a pull request that did. No row shows this; it is
    /// what the retention window is measured against when deciding that a
    /// merged pull request has been merged long enough to stop listing.
    ///
    /// `None` for one that has not merged, and equally for one that has
    /// where the forge did not report a time or reported one that will not
    /// parse. The three are the same answer to the only question asked of
    /// this field - is there a merge time to measure - and collapsing them
    /// keeps the caller from having to distinguish cases that all mean
    /// "keep the record".
    pub merged_at: Option<OffsetDateTime>,
}

impl PullRequestState {
    /// Whether this pull request merged, and merged longer ago than
    /// `retention`.
    ///
    /// The question the retention window asks, answered where the merge time
    /// lives rather than at the call site: the caller then needs no date
    /// library and no opinion about how a forge spells a timestamp.
    ///
    /// False whenever the evidence is missing - not merged, or merged with no
    /// merge time the forge reported or one that would not parse. Knot drops
    /// what it has observed and declines to guess at the rest.
    ///
    /// A merge time ahead of this machine's clock gives a negative span,
    /// which is not a [`Duration`] and so is not past the window. That falls
    /// out of the conversion rather than needing a guard.
    #[must_use]
    pub fn merged_longer_than(&self, retention: Duration, now: SystemTime) -> bool {
        if self.status != PullRequestStatus::Merged {
            return false;
        }
        let Some(merged_at) = self.merged_at
        else {
            return false;
        };
        (OffsetDateTime::from(now) - merged_at).try_into()
                                               .is_ok_and(|since: Duration| since > retention)
    }
}

/// Read the state of the pull request at `url` through the real `gh`.
pub fn pull_request_state(url: &str) -> Result<PullRequestState> {
    pull_request_state_with(&GhRunner::new(), url)
}

/// [`pull_request_state`], against a supplied runner.
///
/// Asks twice at most. GitHub computes mergeability lazily, so the first ask
/// for an open pull request usually answers "unknown" and starts the work;
/// one short retry turns that into a single visible fetch rather than a row
/// that stays uncoloured until the next refresh. A pull request that is not
/// open is never re-asked: there is nothing left to compute.
pub fn pull_request_state_with(runner: &impl ForgeRunner, url: &str) -> Result<PullRequestState> {
    let state = fetch(runner, url)?;
    if !state.status.is_open() || state.mergeable != Mergeability::Unknown {
        return Ok(state);
    }
    std::thread::sleep(MERGEABILITY_RETRY_DELAY);
    // A failed retry is not a failed fetch: the answer already in hand is
    // good except for one field, and losing the row over it would be worse
    // than an uncoloured border.
    Ok(fetch(runner, url).unwrap_or(state))
}

fn fetch(runner: &impl ForgeRunner, url: &str) -> Result<PullRequestState> {
    let stdout = runner.run(&["pr", "view", url, "--json", PULL_REQUEST_FIELDS])?;
    parse_pull_request_state(&stdout)
}

/// What `gh pr view --json` sends back.
///
/// `deny_unknown_fields` is deliberately absent: `gh` adds fields between
/// versions, and a new one it volunteered is not a reason to lose the row.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPullRequest {
    #[serde(default)]
    number:              Option<u64>,
    #[serde(default)]
    title:               Option<String>,
    #[serde(default)]
    state:               Option<String>,
    #[serde(default)]
    is_draft:            Option<bool>,
    #[serde(default)]
    mergeable:           Option<String>,
    #[serde(default)]
    status_check_rollup: Option<Vec<RawCheck>>,
    /// RFC 3339, and absent on a `gh` too old to report it.
    #[serde(default)]
    merged_at:           Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCheck {
    /// Set for a check run: `COMPLETED`, `IN_PROGRESS`, `QUEUED`.
    #[serde(default)]
    status:     Option<String>,
    /// Set for a completed check run: `SUCCESS`, `FAILURE`, `SKIPPED`, ...
    #[serde(default)]
    conclusion: Option<String>,
    /// Set for a commit status rather than a check run: `SUCCESS`, `FAILURE`,
    /// `PENDING`. The two shapes ride in the same array.
    #[serde(default)]
    state:      Option<String>,
}

/// Turn one `gh pr view --json` payload into a state.
pub fn parse_pull_request_state(json: &str) -> Result<PullRequestState> {
    let raw: RawPullRequest =
        serde_json::from_str(json).map_err(|err| ForgeError::Parse(err.to_string()))?;

    let status = status_from(raw.state.as_deref(), raw.is_draft)?;

    let checks = raw.status_check_rollup.as_deref().and_then(rollup_from);

    Ok(PullRequestState { number: raw.number,
                          title: raw.title,
                          status,
                          checks,
                          mergeable: mergeability_from(status, raw.mergeable.as_deref(), checks),
                          merged_at: merged_at_from(status, raw.merged_at.as_deref()) })
}

/// The merge time, for a pull request that merged and reported one that
/// parses.
///
/// Gated on the status so a forge that volunteers `mergedAt` on something it
/// also calls closed cannot produce a state that is both. Status is the field
/// that cannot be absent; this one follows it.
fn merged_at_from(status: PullRequestStatus, merged_at: Option<&str>) -> Option<OffsetDateTime> {
    if status != PullRequestStatus::Merged {
        return None;
    }
    // An unparseable timestamp is dropped rather than raised: it costs a
    // record that does not expire, where failing the parse would cost the
    // whole row.
    merged_at.and_then(|raw| OffsetDateTime::parse(raw, &Rfc3339).ok())
}

/// Fold GitHub's answer, the check rollup and draft-ness into the one
/// question a row asks: could this land right now.
///
/// Draft counts as blocked - GitHub refuses to merge a draft, which is the
/// whole point of marking one. Failing checks count too: a pull request whose
/// CI is red is not one anybody is about to merge, whatever the API says
/// about conflicts.
fn mergeability_from(status: PullRequestStatus, mergeable: Option<&str>,
                     checks: Option<CheckRollup>)
                     -> Mergeability {
    if !status.is_open() {
        return Mergeability::Unknown;
    }
    if status == PullRequestStatus::Draft || checks == Some(CheckRollup::Failing) {
        return Mergeability::Blocked;
    }
    match mergeable.map(str::to_ascii_uppercase).as_deref() {
        Some("MERGEABLE") => Mergeability::Mergeable,
        Some("CONFLICTING") => Mergeability::Blocked,
        // `UNKNOWN`, an absent field, or a value a newer `gh` invented.
        _ => Mergeability::Unknown,
    }
}

/// The one field that cannot be absent. A row that does not know whether its
/// pull request is open has nothing to say, and guessing "open" would show a
/// merged pull request as outstanding - the exact thing not persisting state
/// exists to prevent.
fn status_from(state: Option<&str>, is_draft: Option<bool>) -> Result<PullRequestStatus> {
    let Some(state) = state
    else {
        return Err(ForgeError::Parse("no state field".to_owned()));
    };

    match state.to_ascii_uppercase().as_str() {
        "MERGED" => Ok(PullRequestStatus::Merged),
        "CLOSED" => Ok(PullRequestStatus::Closed),
        "OPEN" if is_draft == Some(true) => Ok(PullRequestStatus::Draft),
        "OPEN" => Ok(PullRequestStatus::Open),
        other => Err(ForgeError::Parse(format!("unknown state {other}"))),
    }
}

/// Collapse the per-check array into the one word a row has space for.
///
/// Any failure fails the rollup; otherwise anything unfinished makes it
/// pending; otherwise it passes. An empty array means no checks are
/// configured, which is not a result - the row shows nothing rather than a
/// green tick nobody earned.
fn rollup_from(checks: &[RawCheck]) -> Option<CheckRollup> {
    if checks.is_empty() {
        return None;
    }

    let mut pending = false;
    for check in checks {
        match check.outcome() {
            Some(CheckRollup::Failing) => return Some(CheckRollup::Failing),
            Some(CheckRollup::Pending) | None => pending = true,
            Some(CheckRollup::Passing) => {}
        }
    }

    Some(if pending {
             CheckRollup::Pending
         }
         else {
             CheckRollup::Passing
         })
}

impl RawCheck {
    /// A check run reports `status` then `conclusion`; a commit status reports
    /// `state`. Both shapes arrive in the same array, so both are read.
    fn outcome(&self) -> Option<CheckRollup> {
        if let Some(status) = &self.status
           && !status.eq_ignore_ascii_case("COMPLETED")
        {
            return Some(CheckRollup::Pending);
        }

        let verdict = self.conclusion.as_deref().or(self.state.as_deref())?;
        match verdict.to_ascii_uppercase().as_str() {
            "SUCCESS" | "NEUTRAL" | "SKIPPED" => Some(CheckRollup::Passing),
            "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "ERROR" => {
                Some(CheckRollup::Failing)
            }
            "PENDING" | "EXPECTED" | "QUEUED" | "IN_PROGRESS" => Some(CheckRollup::Pending),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
