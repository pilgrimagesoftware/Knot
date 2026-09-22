//! Reading one pull request's current state.

use serde::Deserialize;

use crate::consts::PULL_REQUEST_FIELDS;
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
    pub number: Option<u64>,
    pub title:  Option<String>,
    pub status: PullRequestStatus,
    pub checks: Option<CheckRollup>,
}

/// Read the state of the pull request at `url` through the real `gh`.
pub fn pull_request_state(url: &str) -> Result<PullRequestState> {
    pull_request_state_with(&GhRunner::new(), url)
}

/// [`pull_request_state`], against a supplied runner.
pub fn pull_request_state_with(runner: &impl ForgeRunner, url: &str) -> Result<PullRequestState> {
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
    status_check_rollup: Option<Vec<RawCheck>>,
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

    Ok(PullRequestState { number: raw.number,
                          title: raw.title,
                          status,
                          checks: raw.status_check_rollup.as_deref().and_then(rollup_from) })
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
