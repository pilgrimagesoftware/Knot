//! Whether the forge can be read from at all.
//!
//! Probed once per view opening rather than once per pull request, so twenty
//! rows do not mean twenty `gh auth status` calls.

use crate::consts::UNAUTHENTICATED_MARKERS;
use crate::error::ForgeError;
use crate::runner::{ForgeRunner, GhRunner};

/// What the view's single availability message is derived from.
///
/// The three are kept apart because they have different fixes: install `gh`,
/// run `gh auth login`, or look at why a request failed. Collapsing them into
/// one "unavailable" would leave the user without the next step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForgeAvailability {
    /// No `gh` binary on `PATH`.
    Missing,
    /// `gh` is installed but holds no credentials.
    Unauthenticated,
    /// `gh` is installed and authenticated.
    Ready,
    /// `gh` is installed and answered, but not in a way that says either. The
    /// message carries what it said, because there is nothing more specific
    /// to offer.
    Failed(String),
}

impl ForgeAvailability {
    /// Whether state can be fetched. Everything else still lists and still
    /// opens in a browser.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Probe the real `gh`.
#[must_use]
pub fn probe() -> ForgeAvailability {
    probe_with(&GhRunner::new())
}

/// [`probe`], against a supplied runner.
pub fn probe_with(runner: &impl ForgeRunner) -> ForgeAvailability {
    match runner.run(&["auth", "status"]) {
        Ok(_) => ForgeAvailability::Ready,
        Err(ForgeError::Missing) => ForgeAvailability::Missing,
        Err(ForgeError::Unauthenticated) => ForgeAvailability::Unauthenticated,
        Err(ForgeError::Command { output, .. }) if says_unauthenticated(&output) => {
            ForgeAvailability::Unauthenticated
        }
        Err(err) => ForgeAvailability::Failed(err.to_string()),
    }
}

/// `gh auth status` exits non-zero both when it has no credentials and when
/// something else went wrong, so the text is what separates them. Matched
/// loosely and case-insensitively: the wording has changed between `gh`
/// versions, and guessing "failed" for a machine that is merely logged out
/// sends the user looking for the wrong problem.
fn says_unauthenticated(output: &str) -> bool {
    let lowered = output.to_lowercase();
    UNAUTHENTICATED_MARKERS.iter()
                           .any(|marker| lowered.contains(marker))
}

#[cfg(test)]
mod tests;
