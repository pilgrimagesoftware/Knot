//! Finding the command-line tools Knot shells out to when the process's own
//! `PATH` does not name them.
//!
//! A macOS app launched from Finder inherits launchd's `PATH`, which is
//! `/usr/bin:/bin:/usr/sbin:/sbin` and nothing else. Every tool installed by
//! Homebrew, cargo, npm or pip is therefore invisible to it, while the same
//! binary is on `PATH` for every shell the user has ever opened - so the app
//! reports a tool missing on a machine that has it.
//!
//! The fix is one merged search path, used by every subsystem that spawns a
//! tool: the process's own `PATH` first, so a shell-launched app keeps the
//! user's exact ordering, then the standard install locations from
//! [`crate::consts::EXEC_PATH_FALLBACK_DIRS`].
//!
//! Shared here rather than per crate because a tool found by one subsystem
//! and not by another is the confusing half-failure this exists to prevent:
//! `knot-agent-launch` spawns ACP adapters, `knot-forge` spawns `gh`, and
//! both mean the same thing by "installed".

use std::path::{Path, PathBuf};

use crate::consts::EXEC_PATH_FALLBACK_DIRS;

/// The search path Knot spawns tools with, built from this process's
/// environment.
#[must_use]
pub fn search_path() -> String {
    search_path_for(&std::env::var("PATH").unwrap_or_default(),
                    &std::env::var("HOME").unwrap_or_default())
}

/// Pure form of [`search_path`] for tests and callers already holding the
/// values.
///
/// `process_path` entries win the ordering; `~`-prefixed fallbacks are
/// expanded against `home` (kept literal when `home` is empty, matching how
/// a child process would see an unset `HOME`); an entry already named is not
/// duplicated.
#[must_use]
pub fn search_path_for(process_path: &str, home: &str) -> String {
    let fallbacks: Vec<String> = EXEC_PATH_FALLBACK_DIRS.iter()
                                                        .map(|dir| expand_home(dir, home))
                                                        .collect();

    let mut merged: Vec<&str> = Vec::new();
    for entry in process_path.split(':')
                             .chain(fallbacks.iter().map(String::as_str))
    {
        if entry.is_empty() {
            continue;
        }
        if !merged.contains(&entry) {
            merged.push(entry);
        }
    }
    merged.join(":")
}

/// The absolute path `program` resolves to on [`search_path`], or `None`
/// when no directory on it holds an executable by that name.
///
/// Setting `PATH` on the child would in fact be enough on its own today:
/// `std` deliberately avoids `posix_spawn` when the program is a bare name
/// and `PATH` was set, falling back to `fork`/`exec` with the new
/// environment in place so `execvp` searches it. That is std's
/// implementation talking, though, not its documented contract - and the
/// underlying C function searches the *calling* process's `PATH`.
///
/// Resolving first does not rely on it, and buys something either way:
/// "not installed" becomes a decision this crate made about a list of
/// directories it can name, rather than an inference from a spawn error
/// that `NotFound` could also mean something else about.
///
/// A `program` that already contains a separator is returned unchanged -
/// it names a file, not something to look up.
#[must_use]
pub fn resolve_program(program: &str) -> Option<PathBuf> {
    resolve_program_on(&search_path(), program)
}

/// [`resolve_program`], against a supplied search path.
#[must_use]
pub fn resolve_program_on(search_path: &str, program: &str) -> Option<PathBuf> {
    if program.is_empty() {
        return None;
    }
    if program.contains('/') {
        return Some(PathBuf::from(program));
    }

    search_path.split(':')
               .filter(|dir| !dir.is_empty())
               .map(|dir| Path::new(dir).join(program))
               .find(|candidate| is_executable_file(candidate))
}

/// Expands a leading `~` against `home`, leaving the entry literal when
/// there is no home directory to expand against.
fn expand_home(dir: &str, home: &str) -> String {
    match dir.strip_prefix('~') {
        Some(suffix) if !home.is_empty() => format!("{home}{suffix}"),
        _ => dir.to_owned(),
    }
}

/// Whether `candidate` is something that can actually be spawned.
///
/// Metadata is read through the symlink, not at it: Homebrew installs every
/// binary as a link into its Cellar, so testing the link itself would reject
/// the most common install there is.
#[cfg(unix)]
fn is_executable_file(candidate: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(candidate).is_ok_and(|meta| {
                                    meta.is_file() && meta.permissions().mode() & 0o111 != 0
                                })
}

#[cfg(not(unix))]
fn is_executable_file(candidate: &Path) -> bool {
    std::fs::metadata(candidate).is_ok_and(|meta| meta.is_file())
}

#[cfg(test)]
mod tests;
