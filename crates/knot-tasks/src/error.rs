//! The error model for committing and driving a task graph.

use thiserror::Error;

use crate::task::TaskId;

pub type Result<T> = std::result::Result<T, TaskError>;

/// Why a graph could not be committed, or a task could not be moved.
///
/// Every variant names the offending task, because "invalid graph" tells a
/// caller nothing it can act on.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TaskError {
    #[error("task {0} is listed twice")]
    DuplicateId(TaskId),

    #[error("task {task} depends on {missing}, which is not in this plan")]
    UnknownDependency { task: TaskId, missing: TaskId },

    #[error("task {0} depends on itself")]
    SelfDependency(TaskId),

    #[error("these tasks depend on each other in a cycle: {}", join(.0))]
    Cycle(Vec<TaskId>),

    #[error("a plan may hold at most {limit} tasks, not {found}")]
    TooManyTasks { found: usize, limit: usize },

    #[error("task {0} is not in this plan")]
    NotFound(TaskId),

    #[error("task {task} is {state}, so it cannot be dispatched")]
    NotReady { task: TaskId, state: &'static str },

    #[error("task {task} is waiting on {}", join(.unmet))]
    DependenciesUnmet { task: TaskId, unmet: Vec<TaskId> },

    #[error("task {task} is {state}, so its outcome cannot be reported")]
    NotDispatched { task: TaskId, state: &'static str },
}

fn join(ids: &[TaskId]) -> String {
    ids.iter()
       .map(TaskId::as_str)
       .collect::<Vec<_>>()
       .join(", ")
}
