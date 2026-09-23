//! Which filesystem changes under a working tree are worth a refresh.
//!
//! `knot_watch::Watch` takes its relevance predicate from the caller, so the
//! rule lives here rather than in the crate - the same split
//! `knot-discovery` uses for the source-folder watch, which answers a
//! different question and shares nothing with this one.
//!
//! Without a filter the watch is useless rather than merely noisy: git
//! rewrites `.git/objects`, `.git/logs` and `COMMIT_EDITMSG` throughout its
//! own operations, so every stage would re-trigger the refresh that follows
//! it.
//!
//! Contract: `openspec/specs/file-watching/spec.md`.

use std::path::{Component, Path};

/// The `.git` entries that change what `git status` reports: the index
/// (staging), `HEAD` (commits, checkouts, branch switches), and refs (branch
/// and tag updates).
const GIT_DIR: &str = ".git";
const WATCHED_GIT_ENTRIES: &[&str] = &["index", "HEAD"];
const WATCHED_GIT_PREFIX: &str = "refs";

/// A dotfile that is relevant anyway: editing it changes which paths are
/// untracked, which is exactly what the panel lists.
const GITIGNORE: &str = ".gitignore";

/// Whether a changed path could change what `git status` reports.
///
/// Inside `.git`, only the index, `HEAD` and anything under `refs/`. Outside
/// it, everything except dotfiles - and `.gitignore` is not treated as a
/// dotfile, because it decides what counts as untracked.
// UNWIRED: the watch is started in task group 8.2; until then this is
// reached only from tests. Removed once the panel starts its watch.
#[allow(dead_code)]
pub(crate) fn is_relevant(path: &Path) -> bool {
    match git_dir_tail(path) {
        Some(tail) => is_relevant_git_entry(tail),
        None => !is_ignored_dotfile(path),
    }
}

/// The part of `path` after its `.git` component, or `None` when it has none.
fn git_dir_tail(path: &Path) -> Option<&Path> {
    let mut components = path.components();
    while let Some(component) = components.next() {
        if component.as_os_str() == GIT_DIR {
            return Some(components.as_path());
        }
    }
    None
}

fn is_relevant_git_entry(tail: &Path) -> bool {
    let Some(Component::Normal(first)) = tail.components().next()
    else {
        // `.git` itself, with nothing after it.
        return false;
    };

    WATCHED_GIT_ENTRIES.iter().any(|e| first == *e) || first == WATCHED_GIT_PREFIX
}

/// Whether the final component is a dotfile other than `.gitignore`.
///
/// Editors and the OS write these constantly - `.DS_Store`, swap files, editor
/// state - and none of them is a tracked change.
fn is_ignored_dotfile(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str())
    else {
        return false;
    };

    name != GITIGNORE && name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::is_relevant;

    fn relevant(path: &str) -> bool {
        is_relevant(Path::new(path))
    }

    #[test]
    fn staging_head_and_refs_are_relevant() {
        assert!(relevant("/repo/.git/index"), "staging changes the index");
        assert!(relevant("/repo/.git/HEAD"),
                "a commit or checkout moves HEAD");
        assert!(relevant("/repo/.git/refs/heads/main"));
        assert!(relevant("/repo/.git/refs/tags/v1"));
    }

    /// The reason the filter exists: git rewrites all of these during its own
    /// operations, so without the filter every stage re-triggers itself.
    #[test]
    fn gits_own_churn_is_not_relevant() {
        assert!(!relevant("/repo/.git/objects/ab/cdef"));
        assert!(!relevant("/repo/.git/logs/HEAD"));
        assert!(!relevant("/repo/.git/COMMIT_EDITMSG"));
        assert!(!relevant("/repo/.git/config"));
        assert!(!relevant("/repo/.git/hooks/pre-commit"));
        assert!(!relevant("/repo/.git"),
                "the directory itself is not an entry");
    }

    #[test]
    fn working_tree_files_are_relevant() {
        assert!(relevant("/repo/src/main.rs"));
        assert!(relevant("/repo/README.md"));
        assert!(relevant("/repo/deep/nested/file.txt"));
    }

    #[test]
    fn dotfiles_are_ignored_except_gitignore() {
        assert!(!relevant("/repo/.DS_Store"));
        assert!(!relevant("/repo/src/.hidden.swp"));
        assert!(relevant("/repo/.gitignore"),
                "editing it changes which paths are untracked");
        assert!(relevant("/repo/nested/.gitignore"));
    }

    /// A path with `.git` deeper in it - a submodule, or a worktree's own
    /// `.git` file - is still matched on its `.git` component, so the same
    /// rule applies wherever it sits.
    #[test]
    fn a_nested_git_dir_uses_the_same_rule() {
        assert!(relevant("/repo/vendor/dep/.git/index"));
        assert!(!relevant("/repo/vendor/dep/.git/objects/aa/bb"));
    }

    /// A directory named `.github` must not be mistaken for `.git`, and its
    /// contents are ordinary tracked files.
    #[test]
    fn dot_github_is_not_dot_git() {
        assert!(relevant("/repo/.github/workflows/ci.yml"));
    }
}
