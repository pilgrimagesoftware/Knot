//! The processes section's state, and the decisions that drive its sampler.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "Sampling happens off
//! the render path and only while observed".
//!
//! Nothing here reads the process table on the render path. A render asks
//! [`ProcessSection`] for what the last completed sample left; the sample
//! itself runs on a blocking task and lands here between frames - the same
//! separation [`crate::diff_stats`] enforces for `git diff`, for the same
//! reason.
//!
//! One read serves the whole window: `ps -A` returns the entire machine
//! whether one agent is observed or six, so [`sample_roots`] takes every
//! observed root and walks one table per pass. That is what the `FnOnce`
//! sampler encodes - a second read is not merely discouraged, it does not
//! type-check.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use knot_processes::{DescendantProcess, ProcessTable};
use parking_lot::Mutex;
use uuid::Uuid;

#[cfg(test)]
mod tests;

/// One agent's processes section, as the UI holds it.
///
/// Per-agent state in one struct rather than a map per field: the workspace
/// window already carries a dozen `BTreeMap<Uuid, _>` that must be pruned
/// together, and three of them were missed.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct ProcessSection {
    /// Collapsed until the user opens it, per the spec. Expansion is what
    /// starts the sampler, so a window full of agents costs nothing until
    /// somebody asks.
    expanded:    bool,
    /// The last completed sample for this agent.
    ///
    /// `None` means no sample has landed yet, which the collapsed header
    /// renders as an unknown count rather than as zero. A failed sample
    /// never clears it: the spec keeps the last successful list on screen.
    snapshot:    Option<Vec<DescendantProcess>>,
    /// The last sample's failure, cleared silently by the next success.
    failure:     Option<String>,
    /// PIDs whose termination is in flight, so the row can say so until the
    /// next sample either drops it or shows it still running.
    terminating: BTreeSet<u32>,
}

impl ProcessSection {
    pub(crate) fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Flips the section open or shut, answering the new state.
    pub(crate) fn toggle(&mut self) -> bool {
        self.expanded = !self.expanded;
        self.expanded
    }

    /// The last completed sample, or `None` if none has landed.
    pub(crate) fn processes(&self) -> Option<&[DescendantProcess]> {
        self.snapshot.as_deref()
    }

    /// How many background descendants the last sample found, or `None`
    /// before the first one completes.
    pub(crate) fn background_count(&self) -> Option<usize> {
        self.snapshot.as_ref().map(|processes| {
                                  processes.iter()
                                           .filter(|p| p.activity.is_background())
                                           .count()
                              })
    }

    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    pub(crate) fn is_terminating(&self, pid: u32) -> bool {
        self.terminating.contains(&pid)
    }

    pub(crate) fn mark_terminating(&mut self, pid: u32) {
        self.terminating.insert(pid);
    }

    /// Drops the mark without waiting for a sample - what a refused signal
    /// does, since no sample will ever remove a process that is still there.
    pub(crate) fn clear_terminating(&mut self, pid: u32) {
        self.terminating.remove(&pid);
    }

    /// Records a completed sample, clearing any failure notice.
    pub(crate) fn publish(&mut self, processes: Vec<DescendantProcess>) {
        // A PID the sample no longer reports is gone; one it still reports
        // is still being waited on.
        self.terminating
            .retain(|pid| processes.iter().any(|process| process.pid == *pid));
        self.snapshot = Some(processes);
        self.failure = None;
    }

    /// Records a failed sample, leaving the last successful list in place.
    pub(crate) fn fail(&mut self, message: String) {
        self.failure = Some(message);
    }

    /// Forgets what was sampled, without closing the section.
    ///
    /// What an agent stopping does: the rows described a process tree that no
    /// longer exists, and showing them beside "not running" would be a lie.
    pub(crate) fn clear(&mut self) {
        self.snapshot = None;
        self.failure = None;
        self.terminating.clear();
    }
}

/// What one sampling pass published, for the poll to drain into the sections.
///
/// Written on the sampling task and read on the main thread, so it carries a
/// generation the poll compares against: a repeat of the same pass is not
/// news, and notifying on one would repaint at the sampler's rate whether
/// anything moved or not.
#[derive(Debug, Default)]
pub(crate) struct Published {
    pub(crate) descendants: BTreeMap<Uuid, Vec<DescendantProcess>>,
    pub(crate) failure:     Option<String>,
    pub(crate) generation:  u64,
}

/// The slot a window's sampling task publishes into.
pub(crate) type PublishSlot = Arc<Mutex<Published>>;

/// Walks one already-read table for every observed root.
///
/// Separate from [`sample_roots`] so the walk can be tested against a built
/// table, with no live processes involved.
pub(crate) fn descendants_for(table: &ProcessTable, roots: &BTreeMap<Uuid, u32>)
                              -> BTreeMap<Uuid, Vec<DescendantProcess>> {
    roots.iter()
         .map(|(agent, root)| (*agent, table.descendants(*root)))
         .collect()
}

/// One sampling pass: read the process table once, then walk it per root.
///
/// `sample` is `FnOnce` deliberately. The whole cost of the section being
/// open is the one `ps -A` this performs, and a refactor that read the table
/// per agent would make a window with six expanded sections six times as
/// expensive. Taking the reader by value is what stops that being possible.
pub(crate) fn sample_roots<S>(roots: &BTreeMap<Uuid, u32>, sample: S)
                              -> knot_processes::Result<BTreeMap<Uuid, Vec<DescendantProcess>>>
    where S: FnOnce() -> knot_processes::Result<ProcessTable> {
    if roots.is_empty() {
        return Ok(BTreeMap::new());
    }

    Ok(descendants_for(&sample()?, roots))
}

/// What the window is showing, as the sampler's gate cares about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Showing {
    /// The agent whose pane is on screen, or `None` when a takeover view
    /// (the dashboard, the pull requests list) has replaced it.
    pub(crate) agent: Option<Uuid>,
}

/// Which agents this window should be sampling, and the root to sample each
/// from.
///
/// Empty means the sampler has nothing to do and starts nothing. Three of the
/// spec's four stop triggers are ways for an agent to leave this set: the
/// section collapses, the agent's session ends so it has no root, or a
/// takeover view hides the pane the section lives in. The fourth, the window
/// closing, is the window's tokio runtime being dropped along with it, which
/// takes any pass still in flight with it.
pub(crate) fn observed_roots(sections: &BTreeMap<Uuid, ProcessSection>, showing: Showing,
                             root_of: impl Fn(Uuid) -> Option<u32>)
                             -> BTreeMap<Uuid, u32> {
    let Some(shown) = showing.agent
    else {
        return BTreeMap::new();
    };

    sections.iter()
            .filter(|(agent, section)| **agent == shown && section.expanded)
            .filter_map(|(agent, _)| root_of(*agent).map(|root| (*agent, root)))
            .collect()
}
