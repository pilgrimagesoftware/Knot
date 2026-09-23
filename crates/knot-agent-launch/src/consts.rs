//! Fixed strings the registration prompts embed. Kept in one place per the
//! workspace's constants convention.

/// Sent on first launch (ACP protocol prompt, or the deferred shell-agent
/// registration prompt) to trigger the agent list table and status set.
///
/// One line, like the instructions it is appended to: the shell-agent path
/// types the combined prompt into a terminal and then sends Return.
pub const REGISTRATION_USER_PROMPT: &str = "List the other agents and their projects (no IDs) in a table, then set your status to ready. Register with the knot if you are not in that table.";
