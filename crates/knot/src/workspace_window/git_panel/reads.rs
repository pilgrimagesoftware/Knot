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
use crate::git_panel::state::{DiffOutcome, GitStatusSnapshot, Selection};
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Requests a fresh working-tree status for `id` if the cached one has
    /// aged out.
    pub(in crate::workspace_window) fn refresh_git_status(&mut self, id: Uuid, folder: &str) {
        let Some(writer) = self.git_status
                               .claim_refresh(id, consts::GIT_STATUS_MAX_AGE)
        else {
            return;
        };

        let folder = folder.to_string();
        let _runtime_guard = self.runtime.enter();
        self.runtime
            .spawn_blocking(move || writer.record(read_status(&folder)));
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
