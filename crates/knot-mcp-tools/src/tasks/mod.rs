//! The task tools: committing a plan, dispatching from it under the
//! dependency gate, reporting outcomes, and reading it back.
//!
//! Implements the task-tool requirements in
//! `openspec/specs/mcp-tools/spec.md`, over the graph in `knot-tasks`.

mod dispatch;
mod plan;
mod store;

pub use dispatch::{DispatchContext, complete_task, dispatch_task};
pub use plan::{plan_tasks, task_status};
pub use store::GraphStore;

#[cfg(test)]
mod tests;
