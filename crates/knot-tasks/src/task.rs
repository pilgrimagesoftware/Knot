//! The task itself: its identity, who it is for, and the states it moves
//! through.

use std::fmt;
use std::str::FromStr;

use knot_core::Capabilities;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A task's identifier, unique within its graph.
///
/// Caller-chosen rather than generated: an orchestrator writes a plan in one
/// call and refers to the tasks in it by the names it just used, so the ids
/// have to be the ones it picked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for TaskId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}

/// Who a task is for.
///
/// A plan may record work before deciding who does it, and it may name a
/// capability instead of an agent - resolved against the registry at
/// dispatch, never at planning. Resolving early would let a re-plan quietly
/// re-point a task whose work is already under way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assignee {
    Agent(Uuid),
    Capabilities(Capabilities),
}

/// Where a task has got to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    /// Waiting on something that is not done yet.
    Pending,
    /// Every dependency is done; this may be dispatched now.
    Ready,
    /// Handed to an agent; its outcome has not been reported.
    Dispatched,
    Done,
    Failed,
    /// Something it depended on failed, so it will not run as planned.
    Blocked,
}

impl TaskState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Dispatched => "dispatched",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }

    /// Whether the graph may still recompute this state. A finished or
    /// in-flight task is settled: readiness passes must leave it alone, or
    /// a completed task would be re-run and a dispatched one re-sent.
    pub const fn is_settled(self) -> bool {
        matches!(self,
                 Self::Dispatched | Self::Done | Self::Failed | Self::Blocked)
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An unrecognized task state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTaskState(pub String);

impl fmt::Display for UnknownTaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unrecognized task state {:?}", self.0)
    }
}

impl FromStr for TaskState {
    type Err = UnknownTaskState;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "ready" => Ok(Self::Ready),
            "dispatched" => Ok(Self::Dispatched),
            "done" => Ok(Self::Done),
            "failed" => Ok(Self::Failed),
            "blocked" => Ok(Self::Blocked),
            other => Err(UnknownTaskState(other.to_string())),
        }
    }
}

/// How a dispatched task turned out. Deliberately only two values: the
/// graph records what happened, it does not retry, defer or reschedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Failed,
}

/// What a caller submits: a task before the graph has decided its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpec {
    pub id:         TaskId,
    pub goal:       String,
    pub assignee:   Option<Assignee>,
    pub depends_on: Vec<TaskId>,
}

impl TaskSpec {
    pub fn new(id: impl Into<TaskId>, goal: impl Into<String>) -> Self {
        Self { id:         id.into(),
               goal:       goal.into(),
               assignee:   None,
               depends_on: Vec::new(), }
    }

    pub fn assigned_to(mut self, agent: Uuid) -> Self {
        self.assignee = Some(Assignee::Agent(agent));
        self
    }

    pub fn for_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.assignee = Some(Assignee::Capabilities(capabilities));
        self
    }

    pub fn after(mut self, dependencies: impl IntoIterator<Item = TaskId>) -> Self {
        self.depends_on.extend(dependencies);
        self
    }
}

/// A task in a committed graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id:            TaskId,
    pub goal:          String,
    pub assignee:      Option<Assignee>,
    pub depends_on:    Vec<TaskId>,
    pub state:         TaskState,
    /// The agent a dispatch actually went to, once one has. Distinct from
    /// `assignee`: a capability assignee resolves to a different agent each
    /// time it is dispatched, and the plan should record which one it was.
    pub dispatched_to: Option<Uuid>,
}

impl Task {
    pub(crate) fn from_spec(spec: TaskSpec) -> Self {
        let state = if spec.depends_on.is_empty() {
            TaskState::Ready
        }
        else {
            TaskState::Pending
        };
        Self { id: spec.id,
               goal: spec.goal,
               assignee: spec.assignee,
               depends_on: spec.depends_on,
               state,
               dispatched_to: None }
    }
}

#[cfg(test)]
mod tests;
