//! The one shape both feeds produce.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "Subagents reach Knot
//! by the agent's own reporting".
//!
//! An ACP tool call and a posted hook event are different wire formats
//! carrying the same two facts: a subagent was dispatched, or a subagent
//! finished. Recognizers reduce both to this, so nothing downstream - not the
//! registry, not the ordering, not the render - ever branches on which feed a
//! record came from. A feed-specific variant here would put that branch back
//! and let the two paths drift.

use crate::kind::SubagentKind;
use crate::state::Outcome;
use crate::subagent::SubagentId;

#[cfg(test)]
mod tests;

/// One thing an agent reported about one subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubagentEvent {
    Dispatched {
        id:   SubagentId,
        kind: SubagentKind,
        task: String,
    },
    Completed {
        id:      SubagentId,
        outcome: Outcome,
        /// What the agent said about a failure, when it said anything. Carried
        /// beside the outcome rather than inside it because a success has
        /// nowhere to put one - see [`Outcome::into_state`].
        reason:  Option<String>,
    },
}

impl SubagentEvent {
    /// Which subagent this event is about. Both variants have one, and every
    /// caller needs it before it needs anything else - the registry looks the
    /// record up by it, and a `Completed` naming no known record is dropped.
    pub fn id(&self) -> &SubagentId {
        match self {
            Self::Dispatched { id, .. } | Self::Completed { id, .. } => id,
        }
    }

    /// Builds a dispatch, normalizing the kind the report carried.
    ///
    /// Answers `None` when the task is absent or blank. A dispatch with no
    /// task is the partial-input case the spec requires to fail closed: a row
    /// reading only "discovery" with an empty task tells the user less than no
    /// row, because it asserts something is running without saying what.
    pub fn dispatched(id: impl Into<String>, kind: Option<&str>, task: Option<&str>)
                      -> Option<Self> {
        let task = task.map(str::trim).filter(|task| !task.is_empty())?;

        Some(Self::Dispatched { id:   SubagentId::new(id),
                                kind: SubagentKind::from_reported(kind),
                                task: task.to_owned(), })
    }

    /// Builds a completion, parsing the outcome word.
    ///
    /// Answers `None` for an outcome outside the closed vocabulary rather than
    /// assuming success, so an adapter that grows a third word (`cancelled`,
    /// say) leaves the record running - visibly stale - instead of silently
    /// marking it finished.
    pub fn completed(id: impl Into<String>, outcome: &str, reason: Option<&str>) -> Option<Self> {
        let outcome: Outcome = outcome.parse().ok()?;

        Some(Self::Completed { id: SubagentId::new(id),
                               outcome,
                               reason: reason.map(str::trim)
                                             .filter(|reason| !reason.is_empty())
                                             .map(str::to_owned) })
    }
}
