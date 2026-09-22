//! Tunable limits for the task graph.

/// Most tasks one committed graph may hold.
///
/// A plan is a *small* DAG an orchestrator reasons about and a user reads
/// off a diagram; past this it is a work queue, which is a different thing
/// with different requirements (scheduling, persistence, prioritisation)
/// that `openspec/specs/task-graph/spec.md` explicitly does not take on.
/// The cap makes the graph refuse that job loudly instead of doing it
/// badly.
pub const MAX_TASKS: usize = 64;
