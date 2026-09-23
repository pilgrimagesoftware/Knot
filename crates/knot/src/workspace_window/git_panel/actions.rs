//! What the panel's controls do: open and close it, select a row, and run the
//! staging operations.
//!
//! Every git call here goes to the runtime's blocking pool. Swift ran
//! `stage`, `unstage` and `discard` synchronously on the main actor while
//! only its reads were detached; under GPUI that stalls the render loop, and
//! this crate has a rule against I/O on the render path with a cache built
//! because the rule was broken three times.
//!
//! A successful operation does not write the new state - it forgets the old,
//! so the normal read path produces it. One way in, rather than two that can
//! disagree.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::Context;
use knot_git::Repository;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::git_panel::state::Selection;
use crate::workspace_window::WorkspaceWindow;

/// Where a staging operation reports back. `None` while it is still running,
/// so the poll can tell "in flight" from "finished successfully".
pub(in crate::workspace_window) type GitActionSlot = Arc<Mutex<Option<Result<(), String>>>>;

/// One path-scoped staging operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GitAction {
    Stage,
    Unstage,
    Discard,
    StageAll,
    UnstageAll,
}

impl GitAction {
    /// Whether this throws away work nothing can recover.
    ///
    /// Only discard does. Staging and unstaging are each undone by their
    /// opposite, so confirming them would be friction with nothing behind it.
    pub(super) fn is_destructive(self) -> bool {
        self == Self::Discard
    }

    fn run(self, repo: &Repository, paths: &[&str]) -> knot_git::Result<()> {
        match self {
            Self::Stage => repo.stage(paths),
            Self::Unstage => repo.unstage(paths),
            Self::Discard => repo.discard(paths),
            Self::StageAll => repo.stage_all(),
            Self::UnstageAll => repo.unstage_all(),
        }
    }
}

impl WorkspaceWindow {
    pub(super) fn is_git_panel_open(&self, id: Uuid) -> bool {
        self.git_panel_open.contains(&id)
    }

    /// Opens or closes `id`'s panel, starting or stopping its watch with it.
    pub(in crate::workspace_window) fn toggle_git_panel(&mut self, id: Uuid, folder: &str,
                                                        cx: &mut Context<Self>) {
        if self.git_panel_open.remove(&id) {
            self.stop_git_watch(id);
            self.forget_git_state(id);
        }
        else {
            self.git_panel_open.insert(id);
            self.start_git_watch(id, folder);
        }
        cx.notify();
    }

    pub(super) fn close_git_panel(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self.git_panel_open.remove(&id) {
            self.stop_git_watch(id);
            self.forget_git_state(id);
        }
        cx.notify();
    }

    /// Selects a row, or clears the selection when that row is already the
    /// selected one.
    pub(super) fn select_git_row(&mut self, id: Uuid, selection: Selection,
                                 cx: &mut Context<Self>) {
        if self.git_selection.get(&id) == Some(&selection) {
            self.git_selection.remove(&id);
        }
        else {
            self.git_selection.insert(id, selection);
        }
        cx.notify();
    }

    /// Asks for a fresh status now rather than on the cache's cadence.
    pub(super) fn refresh_git_panel(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.git_status.forget(&id);
        self.git_action_error.remove(&id);
        cx.notify();
    }

    pub(in crate::workspace_window) fn set_git_panel_width(&mut self, id: Uuid, width: f32,
                                                           cx: &mut Context<Self>) {
        let clamped = width.clamp(crate::consts::GIT_PANEL_MIN_WIDTH,
                                  crate::consts::GIT_PANEL_MAX_WIDTH);
        self.git_panel_width.insert(id, clamped);
        cx.notify();
    }

    /// Runs one staging operation off the render path, then invalidates what
    /// the panel knows so the next frame re-reads it.
    ///
    /// The watch is paused for the whole operation and resumed only after the
    /// follow-up read, so the writes git is still flushing when the process
    /// exits do not trigger the refresh that already followed them.
    pub(super) fn run_git_action(&mut self, id: Uuid, folder: &str, action: GitAction,
                                 paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let Some(owned) = utf8_paths(&paths)
        else {
            self.git_action_error
                .insert(id, knot_core::l10n::t("git_panel.error.path_not_utf8"));
            cx.notify();
            return;
        };

        self.git_action_error.remove(&id);
        self.pause_git_watch(id);

        let folder = folder.to_string();
        let outcome: GitActionSlot = Arc::new(Mutex::new(None));
        let reported = Arc::clone(&outcome);

        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn_blocking(move || {
                        let repo = Repository::open(&folder);
                        let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
                        *reported.lock() = Some(action.run(&repo, &borrowed)
                                                      .map_err(|error| error.to_string()));
                    });

        self.pending_git_actions.insert(id, outcome);
        cx.notify();
    }

    /// Drains any finished staging operation, invalidating what the panel
    /// knows and resuming that panel's watch.
    ///
    /// Called from the repaint poll rather than from the blocking task: the
    /// task has no GPUI context, so it cannot touch the caches or redraw.
    /// Returns whether anything landed, which is what tells the poll to
    /// notify.
    pub(in crate::workspace_window) fn drain_git_actions(&mut self) -> bool {
        let finished: Vec<(Uuid, Result<(), String>)> =
            self.pending_git_actions
                .iter()
                .filter_map(|(id, slot)| slot.lock().clone().map(|outcome| (*id, outcome)))
                .collect();

        for (id, outcome) in &finished {
            self.pending_git_actions.remove(id);
            self.resume_git_watch(*id);

            match outcome {
                Ok(()) => {
                    self.invalidate_git_state(*id);
                }
                // A failed action leaves the tree as it was, so nothing is
                // invalidated - showing the tree as though it had applied is
                // exactly the lie to avoid.
                Err(reason) => {
                    let text =
                        knot_core::l10n::t_with("git_panel.action_failed", &[("reason", reason)]);
                    self.git_action_error.insert(*id, text);
                }
            }
        }

        !finished.is_empty()
    }

    /// Commits `message` for `id`, reporting failure back through `outcome`.
    ///
    /// The message is trimmed of surrounding whitespace only: internal blank
    /// lines carry the subject-then-body convention and must survive.
    pub(super) fn commit_git_panel(&mut self, id: Uuid, folder: &str, message: &str,
                                   outcome: crate::commit_window::CommitOutcome) {
        let message = message.trim().to_string();
        let folder = folder.to_string();

        self.pause_git_watch(id);
        // Tracked here as well as in the window: the window closing is what
        // the *user* sees, but the tree behind it still has to be re-read,
        // and the window cannot reach the panel's caches.
        self.pending_git_commits.insert(id, Arc::clone(&outcome));

        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn_blocking(move || {
                        let result = Repository::open(&folder).commit(&message)
                                                              .map_err(|error| error.to_string());
                        *outcome.lock() = Some(result);
                    });
    }

    /// Drains any finished commit, invalidating the panel behind the window
    /// and resuming its watch. Returns whether anything landed.
    ///
    /// A failed commit invalidates nothing - the tree is as it was, and the
    /// window is still open showing why.
    pub(in crate::workspace_window) fn drain_git_commits(&mut self) -> bool {
        let finished: Vec<(Uuid, bool)> =
            self.pending_git_commits
                .iter()
                .filter_map(|(id, slot)| slot.lock().as_ref().map(|result| (*id, result.is_ok())))
                .collect();

        for (id, succeeded) in &finished {
            self.pending_git_commits.remove(id);
            if *succeeded {
                self.finish_commit(*id);
            }
            else {
                self.resume_git_watch(*id);
            }
        }

        !finished.is_empty()
    }
}

/// Every path as an owned `String`, or `None` when any is not valid UTF-8.
///
/// `None` rather than skipping the bad one: a bulk action that quietly left a
/// file behind would report success having not done what it said.
fn utf8_paths(paths: &[PathBuf]) -> Option<Vec<String>> {
    paths.iter()
         .map(|p| p.to_str().map(str::to_owned))
         .collect()
}
