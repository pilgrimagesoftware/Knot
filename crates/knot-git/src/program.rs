//! Which `git` this process runs, and under what `PATH`.
//!
//! Set once, by the application, before any [`crate::Runner`] is built;
//! every runner afterwards picks it up without being told. A process-wide
//! value rather than a parameter because `Repository::open` is called from
//! several crates and from a dozen places, and none of them has an opinion
//! about *which* git - they would all be threading the same value through
//! to the same place.
//!
//! The resolving itself is deliberately not here. This crate depends on
//! `thiserror` and nothing else (see the workspace layout in `AGENTS.md`),
//! and finding a binary on the standard install locations is
//! `knot_core::exec_path`'s job. Keeping the search out of here is what
//! lets the crate stay standalone: it accepts an answer, it does not go
//! looking for one.

use std::ffi::OsString;
use std::sync::OnceLock;

use crate::consts::GIT_PROGRAM;

/// The `git` to run and the `PATH` to run it under.
#[derive(Debug, Clone)]
pub struct GitProgram {
    /// The binary to spawn. A bare name is resolved by the spawn itself,
    /// the way it always was.
    pub program:     OsString,
    /// The `PATH` the child is given, or `None` to let it inherit this
    /// process's. Git runs credential helpers, hooks, `git-lfs` and `ssh`
    /// of its own, and they are subject to the same sparse GUI `PATH` that
    /// makes `program` worth resolving.
    pub search_path: Option<OsString>,
}

static CONFIGURED: OnceLock<GitProgram> = OnceLock::new();

/// Sets the git this process runs. Returns `false` if it was already set,
/// in which case the call did nothing.
///
/// Set-once rather than replaceable: a runner built before the change and
/// one built after would otherwise disagree about what "git" means, and the
/// bug that produces - two git versions answering in one session - is far
/// harder to read than the startup ordering it would buy.
pub fn configure(program: GitProgram) -> bool {
    CONFIGURED.set(program).is_ok()
}

/// What [`crate::Runner`] spawns: whatever was configured, or the bare name
/// this crate has always used.
#[must_use]
pub fn configured() -> GitProgram {
    CONFIGURED.get()
              .cloned()
              .unwrap_or_else(|| GitProgram { program:     OsString::from(GIT_PROGRAM),
                                              search_path: None, })
}

#[cfg(test)]
mod tests;
