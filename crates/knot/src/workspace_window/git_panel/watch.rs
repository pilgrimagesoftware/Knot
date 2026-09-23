//! One working-tree watch per open panel: starting it, stopping it, and the
//! pause that keeps the panel's own writes from triggering it.
//!
//! The relevance predicate is `crate::git_panel::watch`; this is its
//! lifecycle. `knot-watch` had no call site before this - its
//! `GIT_STATUS_DEBOUNCE` and `RESUME_SETTLE` constants were written for this
//! panel and name the Swift `GitFileWatcher` they came from.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use knot_watch::Watch;
use uuid::Uuid;

use crate::git_panel::watch::is_relevant;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Starts watching `folder` for `id`, if it is not already watched.
    ///
    /// `Watch::start` calls `tokio::spawn`, so it has to run inside the
    /// runtime - the UI thread has none of its own, and spawning outside one
    /// panics on the main thread at the moment the panel opens.
    pub(super) fn start_git_watch(&mut self, id: Uuid, folder: &str) {
        if self.git_watches.contains_key(&id) {
            return;
        }

        let dirty = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&dirty);
        let watch = Arc::new(Watch::new(folder,
                                        knot_watch::consts::GIT_STATUS_DEBOUNCE,
                                        is_relevant,
                                        move || flag.store(true, Ordering::SeqCst)));

        let _runtime_guard = self.runtime.enter();
        if let Err(error) = watch.start() {
            // A panel without a watch still works; it just does not follow
            // changes made outside Knot. Worth saying, not worth refusing to
            // open for.
            eprintln!("failed to watch {folder} for git changes: {error}");
            return;
        }

        self.git_watches.insert(id, watch);
        self.git_watch_dirty.insert(id, dirty);
    }

    pub(super) fn stop_git_watch(&mut self, id: Uuid) {
        if let Some(watch) = self.git_watches.remove(&id) {
            watch.stop();
        }
        self.git_watch_dirty.remove(&id);
    }

    /// Suppresses `id`'s watch while the panel writes to the tree.
    pub(super) fn pause_git_watch(&self, id: Uuid) {
        if let Some(watch) = self.git_watches.get(&id) {
            watch.pause();
        }
    }

    /// Resumes `id`'s watch, after the operation *and* the read that follows
    /// it.
    ///
    /// `Watch::resume` drops events for a settle window past the call, which
    /// is what covers the writes git is still flushing as its process exits.
    pub(super) fn resume_git_watch(&self, id: Uuid) {
        if let Some(watch) = self.git_watches.get(&id) {
            watch.resume();
        }
    }

    /// Drains every watch that fired, forgetting that agent's status so the
    /// next frame re-reads it. Returns whether anything fired.
    ///
    /// The callback runs on a tokio task with no GPUI context, so it only
    /// flips a flag; acting on it is the poll's job - the same hand-off
    /// `clipboard_writes` and `exited_sessions` use.
    pub(in crate::workspace_window) fn drain_git_watches(&mut self) -> bool {
        let fired: Vec<Uuid> = self.git_watch_dirty
                                   .iter()
                                   .filter(|(_, flag)| flag.swap(false, Ordering::SeqCst))
                                   .map(|(id, _)| *id)
                                   .collect();

        for id in &fired {
            self.git_status.forget(id);
            self.git_diffs.retain(|key| key.agent != *id);
        }

        !fired.is_empty()
    }

    /// Whether any panel has a read that has landed since this was last
    /// asked, for the repaint poll's dirty check.
    pub(in crate::workspace_window) fn git_panel_needs_repaint(&self) -> bool {
        let status_changed = self.git_status.take_changed();
        let diff_changed = self.git_diffs.take_changed();
        status_changed || diff_changed
    }
}
