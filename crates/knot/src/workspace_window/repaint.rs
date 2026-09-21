//! The repaint poll's two predicates.
//!
//! GPUI redraws on notification, not on a clock, so anything that changes
//! off the main thread - a spinner frame, a streaming panel response, a
//! session changing phase - only reaches the screen if something asks for a
//! repaint. These decide whether this frame is one of those, and are
//! deliberately conservative: notifying every poll would repaint thirty
//! times a second while an agent works.

use super::*;

impl WorkspaceWindow {
    /// Whether the dashboard's working indicators need a repaint now: an
    /// agent in this workspace is Working, and the spinner has moved on
    /// since they were last drawn.
    ///
    /// `working_indicator::render` reads the clock when it renders, and the
    /// dashboard - unlike the panel, which redraws as it streams - redraws
    /// only when something happens. Without this the spinner would sit
    /// frozen on whatever frame the last unrelated event left it, which
    /// reads as an agent that has hung. Gating on the frame count rather
    /// than simply notifying every poll repaints about five times a second
    /// while an agent works, instead of thirty, and not at all while none
    /// does.
    pub(super) fn spinner_repaint_due(&mut self) -> bool {
        let any_working = {
            let store = self.store.lock();
            store.workspaces()
                 .iter()
                 .find(|workspace| workspace.id == self.workspace_id)
                 .is_some_and(|workspace| {
                     workspace.agent_ids
                              .iter()
                              .filter_map(|id| store.agent(*id))
                              .any(|agent| {
                                  agent.activated && agent.state == knot_agents::AgentState::Running
                              })
                 })
        };
        if !any_working {
            return false;
        }
        let frame = working_indicator::spinner_frame();
        std::mem::replace(&mut self.last_spinner_frame, frame) != frame
    }

    /// Whether the selected agent's panel needs a repaint: either its live
    /// session has new events, or its slot changed lifecycle phase since
    /// the last poll.
    ///
    /// The phase half matters because `ensure_panel_session` fills the slot
    /// from a background tokio task. `Ready` carries its own dirty flag,
    /// but `Failed` carries nothing - so before this check, a connection
    /// that failed (a missing API key, a refused handshake) left the pane
    /// showing "Connecting to agent…" indefinitely, making the connect
    /// timeout look like it had never fired when in fact the error was
    /// sitting in the slot, undrawn.
    pub(super) fn panel_needs_repaint(&mut self) -> bool {
        let prompt_results = std::mem::take(&mut *self.panel_prompt_results.lock());
        let prompt_results_changed = !prompt_results.is_empty();
        for (id, prompt_id, result) in prompt_results {
            if let Some(queue) = self.panel_prompt_queues.get_mut(&id) {
                // A prompt the user deleted while it was in flight is
                // simply not there any more; `complete` ignores it.
                prompt_queue::complete(queue, prompt_id, result.is_ok());
            }
        }
        let stats_changed = self.diff_stats_dirty
                                .swap(false, std::sync::atomic::Ordering::SeqCst);
        let panel_states = self.panel_sessions
                               .iter()
                               .filter_map(|(id, slot)| {
                                   let slot = slot.lock();
                                   let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                                   else {
                                       return None;
                                   };
                                   let state_arc = handle.state();
                                   let state = state_arc.lock();
                                   let agent_state = if state.pending_permission.is_some() {
                                       knot_agents::AgentState::Input
                                   }
                                   else if state.turn_active {
                                       knot_agents::AgentState::Running
                                   }
                                   else {
                                       knot_agents::AgentState::Idle
                                   };
                                   Some((*id, agent_state))
                               })
                               .collect::<Vec<_>>();
        {
            let mut store = self.store.lock();
            for (id, state) in panel_states {
                store.set_state(id, state);
            }
        }
        // Every agent with something waiting, not just the selected one.
        // This ran only for `selected_agent`, so a prompt queued behind a
        // background agent's turn sat there until the user happened to
        // click that agent - and the inbox nudge is queued precisely for
        // agents nobody is looking at. `drain_panel_prompt` re-checks the
        // session itself, so there is nothing to gate on here.
        let waiting = self.panel_prompt_queues
                          .iter()
                          .filter(|(_, queue)| !queue.is_empty())
                          .map(|(id, _)| *id)
                          .collect::<Vec<_>>();
        for id in waiting {
            self.drain_panel_prompt(id);
        }
        let Some(id) = self.selected_agent
        else {
            return stats_changed;
        };
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let (phase, events_arrived, turn_active) = {
            let slot = slot.lock();
            let events_arrived = matches!(&*slot,
                                          panel_session::PanelSessionSlot::Ready(handle)
                                          if handle.take_dirty());
            let turn_active = match &*slot {
                panel_session::PanelSessionSlot::Ready(handle) => handle.state().lock().turn_active,
                _ => false,
            };
            (slot.phase(), events_arrived, turn_active)
        };
        let phase_changed = self.panel_phases.insert(id, phase) != Some(phase);
        let indicator_due = turn_active
                            && self.working_indicator_last_repaint.elapsed()
                               >= consts::WORKING_INDICATOR_MIN_REPAINT;
        if indicator_due {
            self.working_indicator_last_repaint = std::time::Instant::now();
        }
        phase_changed || events_arrived || indicator_due || stats_changed || prompt_results_changed
    }
}
