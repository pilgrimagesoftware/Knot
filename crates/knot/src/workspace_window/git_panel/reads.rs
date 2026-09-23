//! Asking git for the panel's status and diffs, off the render path.
//!
//! The same claim-refresh discipline as [`super::super::sessions`]'s
//! `refresh_diff_stats`, for the same reason: GPUI re-renders on every
//! keystroke, and these are subprocesses. A render asks for what has landed
//! and separately claims what has aged out; the work runs on the runtime's
//! blocking pool and a later frame draws the answer.
//!
//! `knot-git` is runtime-agnostic by contract, hence `spawn_blocking` rather
//! than an async call.

use std::path::Path;

use knot_git::Repository;
use uuid::Uuid;

use crate::consts;
use crate::git_panel::state::{DiffKey, DiffOutcome, GitStatusSnapshot, Selection};
use crate::refresh_cache::RefreshWriter;
use crate::workspace_window::WorkspaceWindow;

/// The write half of one claimed status refresh.
type StatusWriter = RefreshWriter<Uuid, GitStatusSnapshot>;
/// The write half of one claimed diff refresh.
type DiffWriter = RefreshWriter<DiffKey, DiffOutcome>;

impl WorkspaceWindow {
    /// Requests a fresh working-tree status for `id` if the cached one has
    /// aged out.
    pub(in crate::workspace_window) fn refresh_git_status(&mut self, id: Uuid, folder: &str) {
        let Some(writer) = self.git_status
                               .claim_refresh(id, consts::GIT_STATUS_MAX_AGE)
        else {
            return;
        };
        self.spawn_status_read(writer, folder);
    }

    /// Requests the diff for `selection` if the cached one has aged out.
    ///
    /// Claimed only for the row the panel is currently showing, so this is
    /// one map lookup per frame rather than one per row.
    pub(in crate::workspace_window) fn refresh_git_diff(&mut self, id: Uuid, folder: &str,
                                                        selection: &Selection) {
        let Some(writer) = self.git_diffs
                               .claim_refresh(selection.key(id), consts::GIT_DIFF_MAX_AGE)
        else {
            return;
        };
        self.spawn_diff_read(writer, folder, selection);
    }

    /// Hands a claimed writer to the blocking pool.
    ///
    /// Separate from the claim so the gap between them is *empty*, not merely
    /// free of early returns today. `claim_refresh` marks the key as
    /// requested before it returns, so a writer dropped without calling
    /// `record` leaves that key reading as freshly-requested, with no value
    /// behind it, until the age expires - a stall rather than a missed
    /// repaint, which is the harder kind to notice because a late value looks
    /// like a slow subprocess.
    ///
    /// **This function must stay infallible.** No `?`, no early return, no
    /// fallible call before `spawn_blocking`. Adding one reintroduces exactly
    /// the gap this shape exists to remove - and it would be a one-line edit
    /// that reviews as harmless.
    fn spawn_status_read(&self, writer: StatusWriter, folder: &str) {
        let folder = folder.to_string();
        let _runtime_guard = self.runtime.enter();
        self.runtime
            .spawn_blocking(move || writer.record(read_status(&folder)));
    }

    /// The diff half of [`Self::spawn_status_read`], under the same
    /// must-stay-infallible rule.
    fn spawn_diff_read(&self, writer: DiffWriter, folder: &str, selection: &Selection) {
        let folder = folder.to_string();
        let selection = selection.clone();
        let _runtime_guard = self.runtime.enter();
        self.runtime
            .spawn_blocking(move || writer.record(read_diff(&folder, &selection)));
    }

    /// Drops everything the panel remembers about `id` - its status, every
    /// diff it has shown, and its selection.
    ///
    /// Both cache maps are written from the render path, so without this
    /// every agent and every file the window has ever shown keeps an entry
    /// for the window's whole life.
    pub(in crate::workspace_window) fn forget_git_state(&mut self, id: Uuid) {
        self.git_status.forget(&id);
        self.git_diffs.retain(|key| key.agent != id);
        self.git_selection.remove(&id);
        self.git_action_error.remove(&id);
        self.git_panel_width.remove(&id);
        self.git_panel_resize.remove(&id);
        self.forget_git_diff_list(id);
    }

    /// Invalidates what the panel knows about `id` after the panel itself
    /// changed the working tree, so the next frame re-reads it.
    ///
    /// Forgetting rather than writing the new state means there is one way
    /// state arrives - the normal read path - instead of two that can
    /// disagree. `diff_stats` goes too, so the agent's dashboard card follows
    /// a commit rather than showing pre-commit counts until it ages out.
    pub(in crate::workspace_window) fn invalidate_git_state(&mut self, id: Uuid) {
        self.git_status.forget(&id);
        self.git_diffs.retain(|key| key.agent != id);
        self.diff_stats.forget(&id);
    }
}

/// Reads a working tree's status, distinguishing "not a repository" from a
/// failure and from a clean tree - three different answers that Swift
/// collapsed into one by swallowing the error and returning an empty status.
fn read_status(folder: &str) -> GitStatusSnapshot {
    if !knot_git::is_working_tree(Path::new(folder)) {
        return GitStatusSnapshot::NotARepository;
    }

    match Repository::open(folder).status() {
        Ok(status) => GitStatusSnapshot::Loaded(Box::new(status)),
        Err(error) => GitStatusSnapshot::Failed(error.to_string()),
    }
}

fn read_diff(folder: &str, selection: &Selection) -> DiffOutcome {
    let Some(path) = selection.path.to_str()
    else {
        return DiffOutcome::Failed(knot_core::l10n::t("git_panel.error.path_not_utf8"));
    };
    let orig = selection.orig_path.as_deref().and_then(Path::to_str);

    match Repository::open(folder).file_diff(path, orig, selection.staged) {
        Ok(Some(diff)) => DiffOutcome::Loaded(Box::new(diff)),
        Ok(None) => DiffOutcome::Absent,
        Err(error) => DiffOutcome::Failed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use knot_git::Runner;

    use super::{read_diff, read_status};
    use crate::git_panel::state::{DiffOutcome, GitStatusSnapshot, Selection};

    fn init_repo(dir: &Path) {
        let run = |args: &[&str]| {
            Runner::new(dir).run(args).unwrap();
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "commit.gpgsign", "false"]);
    }

    fn selection(path: &str, staged: bool) -> Selection {
        Selection { path: PathBuf::from(path),
                    orig_path: None,
                    staged }
    }

    /// The three answers Swift collapsed into one. Its repository layer
    /// swallowed the error and returned an empty status, so a folder that was
    /// not a checkout and a repository that could not be read both rendered
    /// as "Working tree clean".
    #[test]
    fn a_folder_that_is_not_a_checkout_says_so() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(read_status(dir.path().to_str().unwrap()),
                   GitStatusSnapshot::NotARepository);
    }

    #[test]
    fn a_repository_that_cannot_be_read_is_a_failure_not_a_clean_tree() {
        let dir = tempfile::tempdir().unwrap();
        // Exists, so `is_working_tree` says yes, but git cannot use it - the
        // shape a corrupted or half-written checkout takes.
        fs::write(dir.path().join(".git"), "not a gitdir").unwrap();

        let status = read_status(dir.path().to_str().unwrap());

        assert!(matches!(status, GitStatusSnapshot::Failed(_)),
                "a failed read must not be reported as a clean tree, got {status:?}");
    }

    #[test]
    fn a_clean_checkout_loads_an_empty_status() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());

        match read_status(dir.path().to_str().unwrap()) {
            GitStatusSnapshot::Loaded(status) => assert!(status.is_clean()),
            other => panic!("expected a loaded status, got {other:?}"),
        }
    }

    #[test]
    fn a_dirty_checkout_loads_its_entries() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        fs::write(dir.path().join("new.txt"), "hello\n").unwrap();

        match read_status(dir.path().to_str().unwrap()) {
            GitStatusSnapshot::Loaded(status) => {
                assert!(!status.is_clean());
                assert_eq!(status.untracked().count(), 1);
            }
            other => panic!("expected a loaded status, got {other:?}"),
        }
    }

    /// `Absent` is git reporting no change on that side - a finished answer,
    /// and a different one from "no answer has landed yet", which the cache
    /// represents by having no entry at all.
    #[test]
    fn an_unchanged_path_reads_as_absent_rather_than_failed() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        fs::write(dir.path().join("f.txt"), "one\n").unwrap();
        let run = |args: &[&str]| {
            Runner::new(dir.path()).run(args).unwrap();
        };
        run(&["add", "-A"]);
        run(&["commit", "-qm", "init"]);

        let folder = dir.path().to_str().unwrap();
        assert_eq!(read_diff(folder, &selection("f.txt", false)),
                   DiffOutcome::Absent);
    }

    #[test]
    fn a_changed_path_loads_its_diff() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        fs::write(dir.path().join("f.txt"), "one\n").unwrap();
        let run = |args: &[&str]| {
            Runner::new(dir.path()).run(args).unwrap();
        };
        run(&["add", "-A"]);
        run(&["commit", "-qm", "init"]);
        fs::write(dir.path().join("f.txt"), "one\ntwo\n").unwrap();

        let folder = dir.path().to_str().unwrap();
        match read_diff(folder, &selection("f.txt", false)) {
            DiffOutcome::Loaded(diff) => assert_eq!(diff.additions(), 1),
            other => panic!("expected a loaded diff, got {other:?}"),
        }
    }

    /// Each side of a staged-and-modified path reports only its own change.
    /// The panel draws two rows for it and they must not show the same diff.
    #[test]
    fn the_two_sides_of_one_path_read_separately() {
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        fs::write(dir.path().join("f.txt"), "one\n").unwrap();
        let run = |args: &[&str]| {
            Runner::new(dir.path()).run(args).unwrap();
        };
        run(&["add", "-A"]);
        run(&["commit", "-qm", "init"]);

        fs::write(dir.path().join("f.txt"), "one\nstaged\n").unwrap();
        run(&["add", "f.txt"]);
        fs::write(dir.path().join("f.txt"), "one\nstaged\nunstaged\n").unwrap();

        let folder = dir.path().to_str().unwrap();
        let staged = read_diff(folder, &selection("f.txt", true));
        let unstaged = read_diff(folder, &selection("f.txt", false));

        assert_ne!(staged, unstaged, "the two sides must not read the same");
        match (staged, unstaged) {
            (DiffOutcome::Loaded(s), DiffOutcome::Loaded(u)) => {
                assert_eq!(s.additions(), 1);
                assert_eq!(u.additions(), 1);
            }
            other => panic!("expected both sides to load, got {other:?}"),
        }
    }

    #[test]
    fn a_read_against_a_broken_repository_is_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".git"), "not a gitdir").unwrap();

        let outcome = read_diff(dir.path().to_str().unwrap(), &selection("f.txt", false));

        assert!(matches!(outcome, DiffOutcome::Failed(_)),
                "expected a failure, got {outcome:?}");
    }
}
