//! Starting probes and landing their results on a frame.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "Probing is bounded
//! and runs off the render path".
//!
//! Two rules shape everything here, and both are `.claude/rules/rust-
//! structure.md`'s:
//!
//! - **No I/O on the render path.** A probe runs an agent's CLI, which
//!   health-checks every configured server over the network. On the render path
//!   that would run per keystroke.
//! - **Off-thread results must reach a frame.** [`Self::mcp_probe_tick`] is the
//!   single place the results queue is drained, and it is called from
//!   `repaint_poll_tick`'s chain. Nothing else may drain it.
//!
//! Probing is *event-driven, not ticked*. It starts when the section first
//! becomes visible for a running Panel agent, when the user refreshes, and
//! when a delegated terminal exits - never on an interval. An MCP health
//! check crosses the network; putting it on the three-second cadence the
//! process sampler uses would be a different change with a different cost.

use knot_core::ViewMode;
use knot_mcp_probe::{CommandRunner, ProbePlan};
use uuid::Uuid;

use super::state::{McpSection, ProbeClaim};
use crate::workspace_window::WorkspaceWindow;

/// Whether a shown agent may be probed, given its view mode and whether it
/// has a live session.
///
/// A free function for the same reason `processes::session_root` is one: it
/// is the whole gate, and a gate that can only be exercised by standing up a
/// window is a gate nothing checks. Takeover and selection are decided by the
/// caller, which owns them.
pub(crate) fn probes(view_mode: Option<ViewMode>, session_root: Option<u32>) -> bool {
    // Terminal mode is excluded by design, not by accident: those agents
    // have a real PTY, so the agent's own `/mcp` already works there.
    view_mode == Some(ViewMode::Panel) && session_root.is_some()
}

impl WorkspaceWindow {
    /// One tick of the MCP probe, from the repaint poll.
    ///
    /// Answers whether anything landed, so the poll repaints for a result
    /// and not for the ticks that find nothing.
    pub(in crate::workspace_window) fn mcp_probe_tick(&mut self) -> bool {
        let landed = self.drain_mcp_probes();
        self.request_first_mcp_probe();
        self.claim_mcp_probe();

        landed
    }

    /// This agent's section, or `None` if it has never been shown.
    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(in crate::workspace_window) fn mcp_section(&self, agent_id: Uuid) -> Option<&McpSection> {
        self.mcp_sections.get(&agent_id)
    }

    /// Whether a probe for this agent is running now.
    ///
    /// Read from the in-flight set rather than stored on the section: a
    /// claim frees on drop, so an unwind cannot leave the header saying
    /// "checking" forever.
    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(in crate::workspace_window) fn mcp_probing(&self, agent_id: Uuid) -> bool {
        self.mcp_in_flight.lock().contains(&agent_id)
    }

    /// Opens or shuts the section, answering its new state.
    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(in crate::workspace_window) fn toggle_mcp_section(&mut self, agent_id: Uuid) -> bool {
        self.mcp_sections.entry(agent_id).or_default().toggle()
    }

    /// Asks for a probe of this agent: the refresh action, and the
    /// delegated terminal's exit.
    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(in crate::workspace_window) fn request_mcp_probe(&mut self, agent_id: Uuid) {
        self.mcp_sections.entry(agent_id).or_default().request();
    }

    /// Drops everything this agent's section held. Called from session
    /// teardown, where every other per-agent map is pruned.
    pub(in crate::workspace_window) fn forget_mcp_section(&mut self, agent_id: Uuid) {
        self.mcp_sections.remove(&agent_id);
    }

    /// The agent a probe may run for: shown, running, and in Panel mode.
    ///
    /// "Running" is [`Self::agent_session_root`] having an answer, which is
    /// `agent-processes`' own definition - never started, deactivated,
    /// mid-restart and exited all report nothing. Reusing it rather than
    /// reading `AgentState` keeps one meaning of the word: `AgentState`
    /// distinguishes idle from working, which is not the question here.
    ///
    /// Terminal-mode agents are excluded by design rather than by accident -
    /// their own `/mcp` works, because they have a real PTY to type it into.
    fn mcp_probe_target(&self) -> Option<Uuid> {
        if self.view_mode.is_takeover() {
            return None;
        }

        let agent_id = self.selected_agent?;

        // Scoped: `agent_session_root` takes the same lock, and
        // `parking_lot::Mutex` is not reentrant.
        let view_mode = {
            let store = self.store.lock();
            store.agent(agent_id).map(|agent| agent.view_mode)
        };

        probes(view_mode, self.agent_session_root(agent_id)).then_some(agent_id)
    }

    /// Asks for the first probe when a section becomes visible.
    fn request_first_mcp_probe(&mut self) {
        let Some(agent_id) = self.mcp_probe_target()
        else {
            return;
        };

        let section = self.mcp_sections.entry(agent_id).or_default();
        if section.needs_first_probe() {
            section.request();
        }
    }

    /// Starts a probe if one was asked for and none is already running.
    fn claim_mcp_probe(&mut self) {
        let Some(agent_id) = self.mcp_probe_target()
        else {
            return;
        };

        let Some(section) = self.mcp_sections.get_mut(&agent_id)
        else {
            return;
        };

        if !section.take_request() {
            return;
        }

        let Some(plan) = self.mcp_probe_plan(agent_id)
        else {
            return;
        };

        // Claimed before the spawn, the way `claim_process_sample` does:
        // work this slow must not be asked for twice while the first is
        // still running. A request that cannot claim is dropped rather than
        // queued - the running probe answers it too.
        let Some(claim) = ProbeClaim::claim(&self.mcp_in_flight, &self.mcp_results, agent_id)
        else {
            return;
        };

        // `spawn_blocking`, not `spawn`: the probe runs a subprocess and
        // blocks until it has drained its output.
        self.runtime.spawn_blocking(move || {
                        let runner = CommandRunner::new();
                        claim.report(knot_mcp_probe::probe(&runner, &plan));
                    });
    }

    /// What to run for this agent, in its own directory and environment.
    fn mcp_probe_plan(&self, agent_id: Uuid) -> Option<ProbePlan> {
        let (agent_type, folder) = {
            let store = self.store.lock();
            let agent = store.agent(agent_id)?;
            (agent.agent_type.clone(), agent.folder.clone())
        };

        // The same resolver the handover uses, so the probe reads the
        // configuration of the installation the flow would open. Two
        // resolvers would drift invisibly.
        let program = self.mcp_program_for(&agent_type);

        // The same `PATH` the ACP adapter is launched with. Knot started
        // from Finder inherits launchd's, which names no directory any agent
        // CLI is installed in, so a probe run under it would report every
        // CLI missing on a machine that has them all.
        let env = vec![("PATH".to_owned(), knot_core::exec_path::search_path())];

        Some(knot_mcp_probe::plan_for(&agent_type, &folder, env, program.as_deref()))
    }

    /// Moves finished probes into their sections, answering whether any
    /// arrived since the last tick.
    ///
    /// The only drain of `mcp_results`. A second reader would consume a
    /// result this one then reports as absent - the "flag read and then
    /// discarded" shape that has broken this chain before.
    fn drain_mcp_probes(&mut self) -> bool {
        let finished = std::mem::take(&mut *self.mcp_results.lock());
        if finished.is_empty() {
            return false;
        }

        for (agent_id, outcome) in finished {
            self.mcp_sections
                .entry(agent_id)
                .or_default()
                .publish(outcome);
        }

        true
    }
}

#[cfg(test)]
mod tests;
