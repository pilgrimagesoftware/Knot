//! Scans a working tree's `openspec/changes/` directory for un-archived
//! OpenSpec changes, and merges results from a repository's several
//! worktrees into one list.
//!
//! Contract: "Un-archived changes are found in the workspace's folders" in
//! the `workspace-openspec-changes` capability spec under `openspec/specs/`.
//!
//! Pure file-system reads, no git: [`scan`] is a thin wrapper around
//! [`std::fs::read_dir`] and [`merge`] is a flat dedupe by name. The window
//! wires both into the OpenSpec tab's cache and `spawn_blocking`.
//!
//! `filter` holds the search and repository filtering the tab applies to the
//! rows this module scans.

pub(crate) mod filter;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

/// One un-archived OpenSpec change found in a working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChangeEntry {
    pub(crate) name: String,
    pub(crate) dir:  PathBuf,
    /// The first non-empty line after a `## Why` heading in the change's
    /// `proposal.md`, trimmed. `None` when there is no `proposal.md`, no
    /// such heading, or nothing before the next `##` heading.
    pub(crate) why:  Option<String>,
}

/// The un-archived changes directly under `<worktree>/openspec/changes/`,
/// sorted by name.
///
/// A directory entry named `archive` or starting with `.` is skipped, and so
/// is anything that is not a directory. A missing `openspec/changes/`
/// directory is not an error: it yields an empty list, the same as a working
/// tree with an empty one.
pub(crate) fn scan(worktree: &Path) -> Vec<ChangeEntry> {
    let changes_dir = worktree.join("openspec").join("changes");
    let Ok(entries) = fs::read_dir(&changes_dir)
    else {
        return Vec::new();
    };

    let mut changes: Vec<ChangeEntry> = entries.filter_map(Result::ok)
                                               .filter(|entry| entry.path().is_dir())
                                               .filter_map(|entry| {
                                                   let name =
                                                       entry.file_name().into_string().ok()?;
                                                   if name == "archive" || name.starts_with('.') {
                                                       return None;
                                                   }
                                                   let dir = entry.path();
                                                   let why = why_line(&dir);
                                                   Some(ChangeEntry { name, dir, why })
                                               })
                                               .collect();

    changes.sort_by(|left, right| left.name.cmp(&right.name));
    changes
}

/// The `## Why` line a change's `proposal.md` summarises itself with, per
/// [`ChangeEntry::why`].
fn why_line(dir: &Path) -> Option<String> {
    let proposal = fs::read_to_string(dir.join("proposal.md")).ok()?;
    let mut lines = proposal.lines();
    lines.find(|line| line.trim() == "## Why")?;
    lines.map(str::trim)
         .take_while(|line| !line.starts_with("##"))
         .find(|line| !line.is_empty())
         .map(str::to_string)
}

/// Merges one repository's [`scan`] results from several worktrees into one
/// list, sorted by name, keeping the first worktree's entry for a name found
/// in more than one - so Reveal in Finder targets the worktree it was first
/// seen in.
pub(crate) fn merge(per_worktree: impl IntoIterator<Item = Vec<ChangeEntry>>) -> Vec<ChangeEntry> {
    let mut merged: Vec<ChangeEntry> = Vec::new();
    for changes in per_worktree {
        for change in changes {
            if !merged.iter().any(|existing| existing.name == change.name) {
                merged.push(change);
            }
        }
    }
    merged.sort_by(|left, right| left.name.cmp(&right.name));
    merged
}
