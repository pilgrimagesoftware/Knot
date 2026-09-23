//! What a subagent is doing, as a closed vocabulary.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "A subagent is a typed
//! record, not free text" and "A failed subagent is distinguished from a
//! finished one".
//!
//! Deliberately *not* built with `settings_vocabulary!`. That macro exists for
//! a stored setting, where an unrecognized value must degrade to a default so
//! one bad field cannot take a whole document down. This vocabulary has the
//! opposite requirement: there is no default, because the spec says a report
//! that does not resolve to one of these produces no record at all. Defaulting
//! a failure to `Finished` would report a subagent succeeded when the agent
//! said it did not, which is worse than reporting nothing.

use std::fmt;
use std::str::FromStr;

use crate::error::SubagentError;

#[cfg(test)]
mod tests;

/// How a subagent's run ended, before a reason is attached.
///
/// Separate from [`SubagentState`] because this is what a *report* carries -
/// one word - while the state is what the record holds, reason included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Succeeded,
    Failed,
}

impl Outcome {
    pub const ALL: &'static [Self] = &[Self::Succeeded, Self::Failed];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// The state this outcome produces, carrying a reason when it failed.
    ///
    /// A reason on a success is dropped rather than stored: `Finished` has
    /// nowhere to put one, and inventing a field so the type can hold a value
    /// nothing renders is how a struct grows a field nobody reads.
    pub fn into_state(self, reason: Option<String>) -> SubagentState {
        match self {
            Self::Succeeded => SubagentState::Finished,
            Self::Failed => SubagentState::Failed { reason },
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Outcome {
    type Err = SubagentError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // No `_ => Ok(Self::Succeeded)`. An outcome word this crate does not
        // know is not a success, and the caller's job on `Err` is to record
        // nothing - see `Recognizer`, which turns it into `None`.
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            other => Err(SubagentError::UnknownOutcome(other.to_owned())),
        }
    }
}

/// A subagent's state: running, or one of the two ways a run ends.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubagentState {
    Running,
    Finished,
    /// The agent reported an error. `reason` is what it said, when it said
    /// anything - a failure with no stated reason is still a failure, and is
    /// never recorded as `Finished`.
    Failed {
        reason: Option<String>,
    },
}

impl SubagentState {
    /// The word this state is named by, which is what [`Display`] and
    /// [`FromStr`] round-trip. The reason is detail hung off `Failed`, not
    /// part of the vocabulary: two failures with different reasons are the
    /// same state.
    ///
    /// [`Display`]: std::fmt::Display
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Finished => "finished",
            Self::Failed { .. } => "failed",
        }
    }

    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }

    /// The reason this run failed, if it failed and one was given.
    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Failed { reason } => reason.as_deref(),
            Self::Running | Self::Finished => None,
        }
    }
}

impl fmt::Display for SubagentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SubagentState {
    type Err = SubagentError;

    /// Parses the state word alone. `failed` yields a failure with no reason,
    /// because the reason travels beside the word rather than inside it; a
    /// caller holding one attaches it by constructing the variant directly.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "running" => Ok(Self::Running),
            "finished" => Ok(Self::Finished),
            "failed" => Ok(Self::Failed { reason: None }),
            other => Err(SubagentError::UnknownState(other.to_owned())),
        }
    }
}
