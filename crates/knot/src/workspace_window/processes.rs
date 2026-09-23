//! Resolving an agent to the process Knot spawned for its session.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "Every running agent
//! exposes a session root process".
//!
//! Two launch paths own a process each: `sessions` holds the PTY-backed
//! shells, `panel_sessions` the ACP adapters. An agent can hold both at once,
//! because a Panel agent switched to Terminal mid-turn keeps its ACP
//! connection alive beside the new PTY. Which one is the session root is
//! therefore decided by the agent's view mode, not by which map happens to
//! have an entry.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use knot_core::ViewMode;
use uuid::Uuid;

use super::window::WorkspaceWindow;
use crate::agent_processes::{self, ProcessSection, Showing};

/// The agent's session root, given what each launch path reports for it.
///
/// `None` whenever the path that owns this agent has no live process: never
/// started, deactivated, mid-restart, or exited.
pub(crate) fn session_root(view_mode: ViewMode, pty: Option<u32>, adapter: Option<u32>)
                           -> Option<u32> {
    match view_mode {
        ViewMode::Terminal => pty,
        ViewMode::Panel => adapter,
    }
}

impl WorkspaceWindow {
    /// One tick of the processes sampler, from the repaint poll.
    ///
    /// Answers whether a new sample landed, so the poll repaints for it and
    /// not for the twenty-nine ticks a second that find nothing.
    pub(super) fn process_sampling_tick(&mut self) -> bool {
        let landed = self.drain_published_processes();
        let refused = self.drain_process_failures();
        self.claim_process_sample();
        landed || refused
    }

    /// This agent's section, or `None` if it has never been opened.
    pub(super) fn process_section(&self, agent_id: Uuid) -> Option<&ProcessSection> {
        self.process_sections.get(&agent_id)
    }

    /// Opens or shuts the section, answering its new state.
    ///
    /// Expanding is what starts the sampler: the next poll tick sees a
    /// non-empty observed set. Collapsing is what stops it.
    pub(super) fn toggle_process_section(&mut self, agent_id: Uuid) -> bool {
        // Deliberately keeps the snapshot either way. Collapsing used to
        // `clear()` it, on the reasoning that nothing is shown while shut and
        // a stale list would be the first thing drawn on reopening - but the
        // collapsed header now names what is running, so clearing is what put
        // it back to "Counting…" for good. Sampling continues while shut, so
        // there is no staleness left to guard against.
        self.process_sections.entry(agent_id).or_default().toggle()
    }

    /// Drops everything this agent's section held. Called from session
    /// teardown, where every other per-agent map is pruned.
    pub(super) fn forget_process_section(&mut self, agent_id: Uuid) {
        self.process_sections.remove(&agent_id);
    }

    /// Which agents this window is sampling right now.
    fn observed_process_roots(&self) -> BTreeMap<Uuid, u32> {
        let showing = Showing { agent: if self.view_mode.is_takeover() {
                                    None
                                }
                                else {
                                    self.selected_agent
                                }, };

        agent_processes::observed_roots(showing, |agent| self.agent_session_root(agent))
    }

    /// Moves a completed pass into the sections, answering whether one had
    /// arrived since the last tick.
    fn drain_published_processes(&mut self) -> bool {
        let (descendants, failure, generation) = {
            let published = self.process_publish.lock();
            if published.generation == self.process_generation {
                return false;
            }
            (published.descendants.clone(), published.failure.clone(), published.generation)
        };
        self.process_generation = generation;

        // An agent whose section has never been toggled still has a header to
        // fill, and `toggle` is the only other thing that creates an entry.
        // Without this the very agents the fix is for - the ones nobody has
        // expanded - would go on showing nothing.
        for agent in descendants.keys() {
            self.process_sections.entry(*agent).or_default();
        }

        for (agent, section) in &mut self.process_sections {
            match (failure.as_ref(), descendants.get(agent)) {
                // A failed sample keeps the last successful list on screen.
                (Some(message), _) => section.fail(message.clone()),
                (None, Some(processes)) => section.publish(processes.clone()),
                // Observed by nobody this pass: shut, or the agent stopped.
                (None, None) => section.clear(),
            }
        }

        true
    }

    /// Starts a sample if one is due and none is already running.
    fn claim_process_sample(&mut self) {
        let roots = self.observed_process_roots();

        if roots.is_empty() {
            // Nothing observed: no subprocess, and the next expansion
            // samples immediately rather than mid-interval.
            self.process_sampled_at = None;
            return;
        }

        if self.process_sampling.load(Ordering::Acquire) {
            return;
        }

        let due = self.process_sampled_at
                      .is_none_or(|at| at.elapsed() >= knot_processes::consts::SAMPLE_INTERVAL);
        if !due {
            return;
        }

        // Marked before the spawn, as `claim_refresh` does: work this slow
        // must not be asked for twice while the first is still running.
        self.process_sampled_at = Some(Instant::now());
        self.process_sampling.store(true, Ordering::Release);

        let slot = Arc::clone(&self.process_publish);
        let running = Arc::clone(&self.process_sampling);
        // `spawn_blocking`, not `spawn`: `knot_processes::sample` runs `ps`
        // and blocks until it has drained its output.
        self.runtime.spawn_blocking(move || {
                        let outcome = agent_processes::sample_roots(&roots, knot_processes::sample);
                        {
                            let mut published = slot.lock();
                            published.generation += 1;
                            match outcome {
                                Ok(descendants) => {
                                    published.descendants = descendants;
                                    published.failure = None;
                                }
                                Err(error) => published.failure = Some(error.to_string()),
                            }
                        }
                        running.store(false, Ordering::Release);
                    });
    }

    /// The process whose descendants this agent's processes section lists.
    pub(super) fn agent_session_root(&self, agent_id: Uuid) -> Option<u32> {
        let view_mode = self.store
                            .lock()
                            .agent(agent_id)
                            .map(|agent| agent.view_mode)?;

        session_root(view_mode,
                     self.terminal_root(agent_id),
                     self.adapter_root(agent_id))
    }

    fn terminal_root(&self, agent_id: Uuid) -> Option<u32> {
        self.sessions
            .get(&agent_id)
            .and_then(|session| session.lock().process_id())
    }

    fn adapter_root(&self, agent_id: Uuid) -> Option<u32> {
        self.panel_sessions
            .get(&agent_id)
            .and_then(|slot| slot.lock().process_id())
    }
}

#[cfg(test)]
mod tests {
    use knot_core::ViewMode;

    use super::session_root;

    #[test]
    fn a_shell_agent_roots_at_its_pty_child() {
        assert_eq!(session_root(ViewMode::Terminal, Some(4242), None),
                   Some(4242));
    }

    #[test]
    fn a_panel_agent_roots_at_its_adapter() {
        assert_eq!(session_root(ViewMode::Panel, None, Some(1717)), Some(1717));
    }

    #[test]
    fn an_agent_holding_both_roots_at_the_one_its_view_mode_names() {
        // A Panel agent switched to Terminal keeps its ACP connection; the
        // pane the user is looking at is the terminal, and so is the root.
        assert_eq!(session_root(ViewMode::Terminal, Some(4242), Some(1717)),
                   Some(4242));
        assert_eq!(session_root(ViewMode::Panel, Some(4242), Some(1717)),
                   Some(1717));
    }

    #[test]
    fn a_deactivated_agent_has_no_root() {
        assert_eq!(session_root(ViewMode::Terminal, None, None), None);
        assert_eq!(session_root(ViewMode::Panel, None, None), None);
    }

    #[test]
    fn an_agent_mid_restart_has_no_root() {
        // The old process is gone and the new one is not up: the owning path
        // reports nothing, and the other path's leftover is not a substitute.
        assert_eq!(session_root(ViewMode::Terminal, None, Some(1717)), None);
        assert_eq!(session_root(ViewMode::Panel, Some(4242), None), None);
    }
}
