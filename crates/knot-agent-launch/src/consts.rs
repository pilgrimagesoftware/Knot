//! Fixed strings the registration prompts embed. Kept in one place per the
//! workspace's constants convention.

/// Sent on first launch (ACP protocol prompt, or the deferred shell-agent
/// registration prompt) to trigger the agent list table and status set.
pub const REGISTRATION_USER_PROMPT: &str = "List other agents names and project (no ID) in a table based on context then set your status to indicate you are ready to get going. If you don't see yourself in the table, register with the knot.";
