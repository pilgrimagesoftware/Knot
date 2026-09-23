//! The MCP servers section in a Panel-mode agent's pane.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md`.
//!
//! What this module owns: one section's state per agent, when a probe runs,
//! and how a finished probe reaches a frame. What it does not own: reading an
//! agent's CLI, which is `knot-mcp-probe`'s job and deliberately knows
//! nothing about windows, and the rows themselves, which are composed from
//! that crate's inventory plus Knot's own server state.
//!
//! The one thing worth knowing before changing anything here: the section
//! shows a *separate process's* view of the same configuration, not the
//! running agent's live connections. ACP offers nothing better - `session/
//! new`'s `mcpServers` is written once and never read back - so the rows are
//! stamped with when they were taken rather than dressed up as live.

pub(crate) mod probe;
pub(crate) mod rows;
pub(crate) mod state;
pub(crate) mod summary;
