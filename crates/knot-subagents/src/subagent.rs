//! One subagent, as Knot holds it.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "A subagent is a typed
//! record, not free text" and "A subagent's lifecycle is observed, not
//! sampled".
//!
//! The difference from [`knot_processes::DescendantProcess`] is the clock.
//! That is re-read from `ps` every few seconds, so its `elapsed` is whatever
//! the last sample said. Nothing re-reads a subagent: it is a log of events
//! that arrived, so the record owns its own start instant and computes elapsed
//! against a caller-supplied `now`. Passing `now` in rather than calling
//! `Instant::now()` internally is what lets one frame render a whole list
//! against a single consistent clock reading, and what makes the arithmetic
//! testable without sleeping.

use std::fmt;
use std::time::{Duration, Instant};

use crate::kind::SubagentKind;
use crate::state::{Outcome, SubagentState};

#[cfg(test)]
mod tests;

/// A subagent's identity, as the agent that dispatched it named it.
///
/// Knot never mints one. The ACP feed uses the tool call's id and the hook
/// feed uses the identifier the agent posted, because the completion report
/// carries only that - a Knot-side identifier would need a side table mapping
/// one to the other, which is the reporter's identifier with extra steps.
///
/// Unique within one agent's session, not globally: two agents may each
/// dispatch a subagent the adapter numbered `1`. The registry keys on the
/// agent first for exactly that reason.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubagentId(pub String);

impl SubagentId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SubagentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One dispatched subagent and what has become of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Subagent {
    pub id:      SubagentId,
    pub kind:    SubagentKind,
    /// What the agent asked it to do, verbatim. Data, not copy: shown as the
    /// agent wrote it and never looked up for translation.
    pub task:    String,
    pub started: Instant,
    pub state:   SubagentState,
    /// When the run ended, for a record that has ended.
    ///
    /// Not in the field list the change's tasks sketched, and needed: the
    /// spec requires a finished record's elapsed time to stop at completion,
    /// and `started` plus a state word cannot say when that was. Kept beside
    /// `state` rather than inside `SubagentState::Finished` so that the
    /// vocabulary stays a vocabulary - two failures with different end
    /// instants are still one state.
    pub ended:   Option<Instant>,
}

impl Subagent {
    /// A newly dispatched subagent, running as of `started`.
    pub fn dispatched(id: SubagentId, kind: SubagentKind, task: String, started: Instant) -> Self {
        Self { id,
               kind,
               task,
               started,
               state: SubagentState::Running,
               ended: None }
    }

    /// Records the end of this run.
    ///
    /// Idempotent in the sense that matters: completing an already-completed
    /// record overwrites the state but keeps the first end instant, so a
    /// duplicate report cannot stretch a finished subagent's elapsed time.
    pub fn complete(&mut self, outcome: Outcome, reason: Option<String>, at: Instant) {
        self.state = outcome.into_state(reason);
        self.ended.get_or_insert(at);
    }

    /// How long this subagent has been running, or ran for.
    ///
    /// Saturating rather than panicking on a `now` before `started`: the
    /// caller is a render pass reading a clock, and a monotonic clock read on
    /// a different thread is not worth crashing a frame over.
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.ended
            .unwrap_or(now)
            .saturating_duration_since(self.started)
    }

    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }
}
