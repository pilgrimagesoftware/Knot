//! Runs `git` and turns its output into structured data: command runner with
//! timeout, porcelain v2 status parsing, unified-diff parsing, numstat stats,
//! staging/commit operations, branch / ahead-behind queries, and worktree
//! detection / creation.
//!
//! Contract: `openspec/specs/git-operations/spec.md` and
//! `openspec/specs/worktree-management/spec.md`.
//!
//! Requires a `git` binary (>= 2.30; see `README.md`) - on `PATH`, or named
//! by the application through [`program::configure`]. This crate does not
//! search for one: locating a binary on the standard install locations
//! belongs to `knot-core`, and this crate stays free of that dependency.
//! Runtime-agnostic: no async runtime; wrap calls in `spawn_blocking` at an
//! async boundary.

pub mod consts;
pub mod diff;
pub mod error;
pub mod program;
pub mod repository;
pub mod runner;
pub mod stats;
pub mod status;
pub mod worktree;

pub use diff::{DiffLine, FileDiff, Hunk, LineKind};
pub use error::{GitError, Result};
pub use program::{GitProgram, configure};
pub use repository::Repository;
pub use runner::Runner;
pub use stats::{DiffStats, parse_numstat};
pub use status::{ChangeType, FileEntry, RepoStatus};
pub use worktree::{is_working_tree, suggest_worktree_path};
