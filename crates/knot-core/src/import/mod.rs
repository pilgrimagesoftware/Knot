//! Bringing definitions in from tools the user already has: a coding agent's
//! subagent definitions, which become personas, and a Skwad installation's
//! workspaces, agents, personas and bench templates.
//!
//! Contract: `openspec/specs/data-import/spec.md`.
//!
//! Every reader in here is read-only and every import is additive: a record
//! the store already holds is skipped rather than merged or duplicated, so
//! running the same import twice leaves the second run with nothing to do.
//! What counts as "already held" is the record's id where the source carries
//! one and its name where it does not.

pub mod personas;
pub mod result;
pub mod skwad;
pub mod subagents;
pub mod workspaces;

pub use personas::import_definitions;
pub use result::{ImportResult, Unreadable, UnreadableReason};
pub use skwad::SkwadSource;
pub use subagents::{
    SubagentDefinition, SubagentProvider, SubagentScan, SubagentTool, provider,
    supports_subagent_import,
};
pub use workspaces::import_workspaces;
