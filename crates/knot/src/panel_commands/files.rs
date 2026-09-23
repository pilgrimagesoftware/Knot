//! The files an `@` mention completes over: everything in the agent's own
//! working folder that is not ignored.
//!
//! Two ways to enumerate, and which one applies is decided by the folder
//! rather than by configuration:
//!
//! - **A git repository** goes through `knot-git`, which lists tracked files
//!   plus untracked ones `--exclude-standard` does not ignore. That honours
//!   `.gitignore`, the global excludes file and `.git/info/exclude` without
//!   reimplementing any of them, so a `target/` the repository ignores never
//!   reaches the popup and a file created a moment ago does.
//! - **Anything else** gets a filesystem walk with the Swift reference's
//!   exclusion set, which is the same list `FileSearchService` skips.
//!
//! Both are capped, and the cap is reported rather than silently applied:
//! `panel-file-mentions` requires the popup say when a folder was too
//! large, because a mention that cannot be completed is otherwise
//! indistinguishable from a file that does not exist.
//!
//! **This reads the filesystem.** Every caller must be a background task -
//! never a render, and never a keystroke.

use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;

use crate::panel_commands::entry::LookupEntry;
use crate::panel_commands::entry::LookupSource;
use crate::panel_commands::entry::Matcher;

#[cfg(test)]
mod tests;

/// Directory names the walk never descends into.
///
/// The Swift reference's `FileSearchService.excludedDirs`, unchanged: the
/// port is meant to offer the same files, and a list that drifts from it
/// is a difference nobody asked for.
const EXCLUDED_DIRS: &[&str] = &[".git",
                                 "node_modules",
                                 ".build",
                                 "__pycache__",
                                 ".DS_Store",
                                 ".svn",
                                 ".hg",
                                 "Pods",
                                 "DerivedData"];

/// How many paths are offered before the folder is called too large.
///
/// The Swift reference's `maxFiles`. Reached only by a folder that is not
/// a repository, in practice - a repository's ignore rules usually cut it
/// down long before this.
const MAX_FILES: usize = 50_000;

/// What enumerating a folder produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FolderFiles {
    /// Paths relative to the folder, in whatever order enumeration found
    /// them. Ranking is the matcher's job, not this one's.
    paths:    Vec<String>,
    /// Whether the folder held more than [`MAX_FILES`] and the list was
    /// cut. Matching still runs over what was gathered, so the popup stays
    /// useful while saying it is incomplete.
    overflow: bool,
}

impl FolderFiles {
    /// Enumerates `folder`, by git where it is a repository and by walking
    /// where it is not.
    ///
    /// Blocking. Call it from `spawn_blocking` or an equivalent.
    pub(crate) fn enumerate(folder: &Path) -> Self {
        if !folder.is_dir() {
            return Self::empty();
        }
        match enumerate_repository(folder) {
            Some(paths) => Self::capped(paths),
            None => Self::capped(walk(folder)),
        }
    }

    /// Nothing found, and not because of a cap.
    pub(crate) fn empty() -> Self {
        Self { paths:    Vec::new(),
               overflow: false, }
    }

    /// Builds from an already-gathered list, applying the cap.
    fn capped(mut paths: Vec<String>) -> Self {
        let overflow = paths.len() > MAX_FILES;
        paths.truncate(MAX_FILES);
        Self { paths, overflow }
    }

    /// Whether the folder held more files than were gathered.
    pub(crate) fn overflowed(&self) -> bool {
        self.overflow
    }
}

impl LookupSource for FolderFiles {
    fn entries(&self) -> Vec<LookupEntry> {
        // A path is its own description: the popup shows the path, and
        // there is nothing to say about a file that the path does not
        // already say.
        self.paths
            .iter()
            .map(|path| LookupEntry::new(path.clone(), String::new()))
            .collect()
    }

    fn matcher(&self) -> Matcher {
        Matcher::Subsequence
    }
}

/// `folder`'s files by way of git, or `None` when it is not a repository.
fn enumerate_repository(folder: &Path) -> Option<Vec<String>> {
    if !folder.join(".git").exists() {
        return None;
    }
    knot_git::Repository::open(folder).list_files().ok()
}

/// Every file under `folder`, skipping the excluded directory names and
/// anything the filesystem refuses.
///
/// Breadth-first so a cap cuts the deepest paths rather than an arbitrary
/// subtree: the shallower a file, the likelier it is the one being named.
fn walk(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::from([folder.to_path_buf()]);

    while let Some(directory) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&directory)
        else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if EXCLUDED_DIRS.contains(&name.as_ref()) {
                continue;
            }
            match entry.file_type() {
                // Symlinks are not followed: a link pointing at a parent
                // is a walk that does not end.
                Ok(kind) if kind.is_dir() => queue.push_back(path),
                Ok(kind) if kind.is_file() => {
                    if let Ok(relative) = path.strip_prefix(folder) {
                        found.push(relative.to_string_lossy().into_owned());
                    }
                }
                _ => {}
            }
            if found.len() > MAX_FILES {
                return found;
            }
        }
    }
    found
}
