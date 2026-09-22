//! The task graph: a small directed acyclic graph of work an orchestrating
//! agent commits before it dispatches anything nontrivial.
//!
//! Contract: `openspec/specs/task-graph/spec.md`.
//!
//! Standalone on purpose, in the shape of `knot-git` and `knot-discovery`:
//! no async runtime, no UI, no MCP types, and no access to the agent store.
//! The graph names agents by id and capabilities by tag, and resolves
//! neither - both are handed back to the caller, which is where the
//! registry lives. Keeping the crate ignorant of what an agent *is* is what
//! makes the ordering rules testable on their own, and it stops the
//! tempting shortcut of resolving a capability from inside the state
//! machine.
//!
//! A graph is runtime state: it is not persisted, and it does not outlive
//! the process, matching how messages behave in `mcp-messaging`. A plan
//! whose agents are gone is not a plan.

pub mod consts;
mod error;
mod graph;
mod task;

pub use error::{Result, TaskError};
pub use graph::{Completion, Dispatchable, TaskGraph};
pub use task::{Assignee, Outcome, Task, TaskId, TaskSpec, TaskState, UnknownTaskState};
