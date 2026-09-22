//! This crate's error type.
//!
//! The tool handlers themselves return [`ToolCallResult`], which carries a
//! message for the calling agent and nothing an `if let` can branch on. The
//! functions *behind* them return these instead, so a caller can tell a
//! missing repository from a git failure before flattening both into a
//! message - the workspace convention (`GitError`, `DiscoveryError`, core
//! `Error`) applied to the last crate that was still returning `String`.
//!
//! [`ToolCallResult`]: knot_mcp::ToolCallResult

use std::path::PathBuf;

/// What a tool's supporting work can fail with.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// A path an agent named is not a git working tree, so there is nothing
    /// to branch a worktree from.
    #[error("not a git repository: {path}")]
    NotARepository { path: PathBuf },

    /// `git` itself refused, e.g. the branch already has a worktree.
    #[error(transparent)]
    Git(#[from] knot_git::GitError),

    /// The agent roster could not be written back to the settings file, so
    /// an agent this tool created will be missing after a relaunch.
    #[error(transparent)]
    Settings(#[from] knot_core::Error),
}

/// This crate's result alias, as every other crate in the workspace has.
pub type Result<T, E = ToolError> = std::result::Result<T, E>;
