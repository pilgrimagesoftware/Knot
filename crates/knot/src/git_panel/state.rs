//! What the panel knows, and the caches it knows it through.
//!
//! Two caches, because the panel asks git two different questions on two
//! different cadences: one status per agent, and one diff per selected row.
//!
//! The diff cache's key is the point. Swift kept a single `selectedDiff` slot,
//! so a reply for a file the user had already clicked away from overwrote the
//! current one - two quick clicks and the slower reply won. Keying by
//! [`DiffKey`] gives every reply its own entry, and a render only ever reads
//! the entry for the current selection, so a late reply has nowhere to land.
//! The race is absent rather than guarded against.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`.

use std::path::PathBuf;

use knot_git::{FileDiff, RepoStatus};
use uuid::Uuid;

use crate::refresh_cache::RefreshCache;

/// Which agent's panel, which path, and which side of that path.
///
/// The `staged` flag is part of the identity, not a modifier: a path that is
/// staged and then modified again has two rows and two different diffs, and
/// they must not share a cache entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) struct DiffKey {
    pub(crate) agent:  Uuid,
    pub(crate) path:   PathBuf,
    pub(crate) staged: bool,
}

/// What reading a repository's status produced.
///
/// A failure is a value here, not an absent entry. `refresh_cache` reserves
/// absence for "no answer has landed yet", so encoding a failure as absence
/// leaves the panel on its loading state forever.
///
/// The failure carries a `String` rather than a `GitError` because
/// [`RefreshCache`] needs `Clone + PartialEq` and `GitError` is neither. The
/// message is formatted where the error is caught.
#[derive(Debug, Clone, PartialEq, Eq)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) enum GitStatusSnapshot {
    /// The folder exists but is not a working tree. A finished answer, and a
    /// different one from a clean tree.
    NotARepository,
    Failed(String),
    Loaded(Box<RepoStatus>),
}

/// What reading one path's diff produced. `Absent` is git reporting no change
/// on that side, which is a finished answer.
#[derive(Debug, Clone, PartialEq, Eq)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) enum DiffOutcome {
    Failed(String),
    Absent,
    Loaded(Box<FileDiff>),
}

// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) type GitStatusCache = RefreshCache<Uuid, GitStatusSnapshot>;
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) type GitDiffCache = RefreshCache<DiffKey, DiffOutcome>;

/// Which row the panel is showing a diff for.
///
/// Identifies a row the same way [`DiffKey`] does, minus the agent: the
/// selection lives per panel, so the agent is already known. `orig_path` is a
/// rename's source, carried because `Repository::file_diff` needs both sides
/// to see a rename as a rename.
#[derive(Debug, Clone, PartialEq, Eq)]
// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
pub(crate) struct Selection {
    pub(crate) path:      PathBuf,
    pub(crate) orig_path: Option<PathBuf>,
    pub(crate) staged:    bool,
}

// UNWIRED: the panel's element tree lands in task groups 3-9; until then
// this is reached only from tests. Removed once the panel renders.
#[allow(dead_code)]
impl Selection {
    pub(crate) fn key(&self, agent: Uuid) -> DiffKey {
        DiffKey { agent,
                  path: self.path.clone(),
                  staged: self.staged }
    }
}
