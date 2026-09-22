//! The per-agent `git diff --numstat` cache both dashboards read.
//!
//! `git diff` is a subprocess. GPUI re-renders on every keystroke, and a
//! dashboard draws one card per agent, so computing a stat where it is drawn
//! costs one subprocess per agent per frame - which is how the workspace
//! window's header once ran typing at the speed of `git`.
//!
//! So nothing computes a stat on the render path. A render asks this cache
//! for what it already has, and separately asks it to refresh what has aged
//! out; the refresh runs off the main thread and the answer is picked up by
//! a later frame. The cache owns the values, the per-agent request times and
//! the "something changed" flag; *how* the work gets off the main thread is
//! the window's business, because the two windows differ there - the
//! workspace window has a tokio runtime already and a poll that redraws it,
//! the command center has neither and spawns through GPUI instead.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Instant;

use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts;

/// Last known diff stat per agent, and when each was last requested.
///
/// The inner `Option` is the *lookup*: `None` means the folder is not a git
/// checkout, which is a finished answer and not a pending one. An agent
/// absent from the map is the pending case.
#[derive(Default)]
pub(crate) struct DiffStatsCache {
    values:    Arc<Mutex<BTreeMap<Uuid, Option<knot_git::DiffStats>>>>,
    /// Main-thread only: when each agent's stat was last *requested*, so
    /// the refresh runs on a cadence rather than once per render.
    requested: BTreeMap<Uuid, Instant>,
    dirty:     Arc<AtomicBool>,
}

impl DiffStatsCache {
    /// Claims a refresh of `id` if its stat has aged out.
    ///
    /// Returns the writer the caller hands to whatever runs the `git` call,
    /// or `None` when the cached value is still fresh - which is the common
    /// case, since this is asked once per agent per frame. Claiming marks
    /// the request immediately, so a slow `git` cannot be asked for twice.
    pub(crate) fn claim_refresh(&mut self, id: Uuid) -> Option<DiffStatsWriter> {
        let fresh = self.requested
                        .get(&id)
                        .is_some_and(|at| at.elapsed() < consts::DIFF_STATS_MAX_AGE);
        if fresh {
            return None;
        }
        self.requested.insert(id, Instant::now());
        Some(DiffStatsWriter { id,
                               values: Arc::clone(&self.values),
                               dirty: Arc::clone(&self.dirty) })
    }

    /// One agent's cached stat.
    ///
    /// The outer `Option` is whether an answer has landed at all, the inner
    /// one whether it found a repository - a header that conflates them
    /// leaves a folder that is not a checkout on "Getting stats…" forever.
    pub(crate) fn get(&self, id: Uuid) -> Option<Option<knot_git::DiffStats>> {
        self.values.lock().get(&id).copied()
    }

    /// The cached stat per agent, copied out so a render can read it without
    /// holding the lock across the element tree it builds.
    pub(crate) fn snapshot(&self) -> BTreeMap<Uuid, Option<knot_git::DiffStats>> {
        self.values.lock().clone()
    }

    /// Whether a refresh has landed since this was last asked, clearing the
    /// flag. For the poll that decides whether to redraw.
    pub(crate) fn take_changed(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }

    /// Drops everything remembered about `id`.
    ///
    /// Both maps are keyed by agent id and written from the render path, so
    /// without this every agent the window has ever shown keeps an entry for
    /// the window's whole life - including agents that no longer exist.
    pub(crate) fn forget(&mut self, id: Uuid) {
        self.values.lock().remove(&id);
        self.requested.remove(&id);
    }
}

/// The write half of one claimed refresh, sent to whatever thread runs the
/// `git` call. Carries no reference back to the window, so a refresh
/// outliving its window costs nothing.
pub(crate) struct DiffStatsWriter {
    id:     Uuid,
    values: Arc<Mutex<BTreeMap<Uuid, Option<knot_git::DiffStats>>>>,
    dirty:  Arc<AtomicBool>,
}

impl DiffStatsWriter {
    /// Records the result, flagging the cache as changed only when it
    /// actually differs - a stat that came back the same must not cost a
    /// repaint, which is most refreshes.
    pub(crate) fn record(self, stats: Option<knot_git::DiffStats>) {
        if self.values.lock().insert(self.id, stats) != Some(stats) {
            self.dirty.store(true, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests;
