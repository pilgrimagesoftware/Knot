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
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use tokio::time::timeout;

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

    /// How long to keep watching after the expected callback arrives, to
    /// catch a second one. Comfortably past the debounce, since a burst that
    /// failed to collapse would fire again within one.
    fn hold_window() -> Duration {
        knot_watch::consts::GIT_STATUS_DEBOUNCE + Duration::from_millis(500)
    }

    /// Waits for `count` to reach `expected`, then holds to see whether it
    /// goes further. Returns the count after the hold.
    ///
    /// Two phases rather than "sleep, then look", deliberately. A fixed
    /// sample schedule is a trap on a loaded machine: these agents share one
    /// box and it runs at a load average several times its core count, so a
    /// 1s debounce can take well over a second of wall clock to fire. A test
    /// that samples on a timer then reports what it saw blames the watch for
    /// its own impatience - which is exactly what an earlier version of this
    /// helper did with a window shorter than the debounce.
    ///
    /// So phase one waits for the value with a generous budget and does not
    /// care how long it takes, and phase two is what actually asserts the
    /// collapse: having reached one callback, it must not become two.
    async fn reaches_then_holds(count: &AtomicUsize, expected: usize, hold: Duration) -> usize {
        timeout(Duration::from_secs(60), async {
            while count.load(Ordering::SeqCst) < expected {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }).await
          .unwrap_or_else(|_| {
              panic!("the watch never reached {expected} callbacks within 60s (saw {})",
                     count.load(Ordering::SeqCst))
          });

        tokio::time::sleep(hold).await;
        count.load(Ordering::SeqCst)
    }

    /// A working tree with a `.git` directory, so the predicate has both
    /// kinds of path to judge.
    fn work_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
        fs::create_dir_all(dir.path().join(".git/refs/heads")).unwrap();
        dir
    }

    /// The panel's watch under the predicate it actually runs with: fifty
    /// files changing at once is one refresh, not fifty.
    ///
    /// `knot-watch` has its own burst test, but with an always-true
    /// predicate. This one is the pair that ships - the real debounce and
    /// the real relevance filter - because a filter that let each event
    /// through separately would pass that test and fail here.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_burst_of_working_tree_changes_is_one_refresh() {
        let dir = work_tree();
        let count = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&count);

        let watch = knot_watch::Watch::new(dir.path(),
                                           knot_watch::consts::GIT_STATUS_DEBOUNCE,
                                           is_relevant,
                                           move || {
                                               counted.fetch_add(1, Ordering::SeqCst);
                                           });
        watch.start().unwrap();

        for i in 0..50 {
            fs::write(dir.path().join(format!("file{i}.rs")), "changed").unwrap();
        }

        let fired = reaches_then_holds(&count, 1, hold_window()).await;
        watch.stop();

        assert_eq!(fired, 1,
                   "fifty changes at once must collapse to one refresh");
    }

    /// The filter earning its keep: git rewrites these throughout its own
    /// operations, so without it every stage would re-trigger the refresh
    /// that follows it.
    #[tokio::test(flavor = "multi_thread")]
    async fn gits_own_churn_drives_no_refresh() {
        let dir = work_tree();
        let count = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&count);

        let watch = knot_watch::Watch::new(dir.path(),
                                           knot_watch::consts::GIT_STATUS_DEBOUNCE,
                                           is_relevant,
                                           move || {
                                               counted.fetch_add(1, Ordering::SeqCst);
                                           });
        watch.start().unwrap();

        for i in 0..20 {
            fs::write(dir.path().join(format!(".git/objects/obj{i}")), "x").unwrap();
        }
        fs::write(dir.path().join(".git/COMMIT_EDITMSG"), "wip").unwrap();
        fs::write(dir.path().join(".DS_Store"), "junk").unwrap();

        // Absence cannot be waited for the way a value can, so this is a
        // bounded-confidence check: several debounces' worth of room, and if
        // load delays a spurious callback past it the test passes when it
        // should not. That direction is the safe one - it cannot fail
        // spuriously, only under-report - and the two positive tests above
        // prove the predicate is not simply rejecting everything.
        tokio::time::sleep(knot_watch::consts::GIT_STATUS_DEBOUNCE * 4).await;
        watch.stop();

        assert_eq!(count.load(Ordering::SeqCst),
                   0,
                   "git's own churn and dotfiles must not drive a refresh");
    }

    /// The other half of the same filter: a change that does matter still
    /// gets through. Without this, a predicate that always returned false
    /// would pass the churn test above.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_staging_write_does_drive_a_refresh() {
        let dir = work_tree();
        let count = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&count);

        let watch = knot_watch::Watch::new(dir.path(),
                                           knot_watch::consts::GIT_STATUS_DEBOUNCE,
                                           is_relevant,
                                           move || {
                                               counted.fetch_add(1, Ordering::SeqCst);
                                           });
        watch.start().unwrap();

        fs::write(dir.path().join(".git/index"), "staged").unwrap();

        let fired = reaches_then_holds(&count, 1, hold_window()).await;
        watch.stop();

        assert_eq!(fired, 1, "staging changes the index, which is a refresh");
    }
}
