//! Every agent's subagents, and the flag that gets a change onto a frame.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "A subagent's
//! lifecycle is observed, not sampled" and "Subagents are recorded flat".
//!
//! One registry with two writers - the ACP fold on a session task, the hook
//! route on an axum worker - rather than one store per feed. The render and
//! the header summary then never branch on where a record came from, which is
//! the branch that would let the two paths drift.
//!
//! Keyed on the agent first because a subagent's id is unique only within its
//! own session: two agents may each hold one the adapter numbered `1`.
//!
//! Flat within an agent, per the spec. A subagent that itself reports a
//! delegation is recorded as another subagent of the same parent - Claude Code
//! does not currently permit it, and a tree for a case that cannot arise is a
//! shape nothing fills.

use std::collections::BTreeMap;
use std::time::Instant;

use uuid::Uuid;

use crate::consts::MAX_FINISHED_PER_AGENT;
use crate::event::SubagentEvent;
use crate::subagent::Subagent;

#[cfg(test)]
mod tests;

/// Every agent's subagents, plus whether anything has changed since the last
/// frame drew.
#[derive(Debug, Default)]
pub struct SubagentRegistry {
    by_agent: BTreeMap<Uuid, Vec<Subagent>>,
    changed:  bool,
}

impl SubagentRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one reported event to `agent`'s records.
    ///
    /// `at` is when the event is being recorded, supplied rather than read so
    /// a caller replaying a batch can keep one consistent clock.
    pub fn apply(&mut self, agent: Uuid, event: SubagentEvent, at: Instant) {
        match event {
            SubagentEvent::Dispatched { id, kind, task } => {
                let records = self.by_agent.entry(agent).or_default();

                // A repeat dispatch is the adapter refining a call it already
                // announced, not a second subagent. Keeping the original start
                // instant is what stops a refinement from resetting a running
                // subagent's clock to zero.
                if records.iter().any(|record| record.id == id) {
                    return;
                }

                records.push(Subagent::dispatched(id, kind, task, at));
                self.changed = true;
            }
            SubagentEvent::Completed { id,
                                       outcome,
                                       reason, } => {
                // A completion naming a record we never saw dispatched is
                // dropped, per the spec: Knot may have started mid-turn, and
                // inventing a finished row for a subagent whose task and kind
                // are unknown would put a blank line on screen.
                let Some(records) = self.by_agent.get_mut(&agent)
                else {
                    return;
                };
                let Some(record) = records.iter_mut().find(|record| record.id == id)
                else {
                    return;
                };

                record.complete(outcome, reason, at);
                self.changed = true;
                prune_finished(records);
            }
        }
    }

    /// `agent`'s subagents in the order the section lists them: running first,
    /// and longest-running first within each group.
    ///
    /// Sorted on read rather than held sorted, because the ordering depends on
    /// `now` and a stored order would be stale the moment the clock moved.
    #[must_use]
    pub fn ordered(&self, agent: Uuid, now: Instant) -> Vec<&Subagent> {
        let Some(records) = self.by_agent.get(&agent)
        else {
            return Vec::new();
        };

        let mut ordered: Vec<&Subagent> = records.iter().collect();
        ordered.sort_by(|left, right| {
                   // Running before finished, then longest-running first.
                   // `Ordering` is derived from bools rather
                   // than an enum rank so that adding a
                   // third state cannot silently join one of the two groups.
                   right.is_running()
                        .cmp(&left.is_running())
                        .then_with(|| right.elapsed(now).cmp(&left.elapsed(now)))
               });

        ordered
    }

    /// Whether `agent` holds any record at all.
    #[must_use]
    pub fn is_empty_for(&self, agent: Uuid) -> bool {
        self.by_agent
            .get(&agent)
            .is_none_or(|records| records.is_empty())
    }

    #[must_use]
    pub fn running_count(&self, agent: Uuid) -> usize {
        self.by_agent.get(&agent).map_or(0, |records| {
                                     records.iter().filter(|r| r.is_running()).count()
                                 })
    }

    /// Forgets everything recorded for `agent`.
    ///
    /// What a turn ending does, and what an agent stopping or its session
    /// ending does. The records describe work inside a turn that is over;
    /// leaving them up would be the section reporting delegation that is no
    /// longer happening.
    pub fn clear(&mut self, agent: Uuid) {
        if self.by_agent.remove(&agent).is_some_and(|r| !r.is_empty()) {
            self.changed = true;
        }
    }

    /// Whether anything has changed since this was last asked, clearing the
    /// flag.
    ///
    /// Consuming, so exactly one caller may have it: `repaint_poll_tick`.
    /// A second reader would take the change the first one needed and the
    /// frame would never be asked for.
    /// `.claude/rules/rust-structure.md` names four separate breaks of that
    /// chain, none of which failed a test.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }
}

/// Drops the oldest finished records past the cap, leaving running ones alone.
///
/// Oldest by when the run *ended*, which is the order a user stops caring
/// about them in - not by when it started, which would discard a long job that
/// just finished before a short one that finished an hour ago.
fn prune_finished(records: &mut Vec<Subagent>) {
    let finished = records.iter().filter(|record| !record.is_running()).count();
    let Some(excess) = finished.checked_sub(MAX_FINISHED_PER_AGENT)
                               .filter(|n| *n > 0)
    else {
        return;
    };

    let mut ends: Vec<Instant> = records.iter()
                                        .filter(|record| !record.is_running())
                                        .map(|record| record.ended.unwrap_or(record.started))
                                        .collect();
    ends.sort_unstable();
    // Everything that ended at or before this instant is in the excess.
    let cutoff = ends[excess - 1];

    let mut still_to_drop = excess;
    records.retain(|record| {
               if record.is_running() || still_to_drop == 0 {
                   return true;
               }

               if record.ended.unwrap_or(record.started) <= cutoff {
                   still_to_drop -= 1;
                   return false;
               }

               true
           });
}
