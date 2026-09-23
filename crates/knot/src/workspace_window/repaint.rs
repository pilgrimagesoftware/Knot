//! The repaint poll: the loop that runs it, and the two predicates that
//! decide whether a tick has anything to draw.
//!
//! GPUI redraws on notification, not on a clock, so anything that changes
//! off the main thread - a spinner frame, a streaming panel response, a
//! session changing phase - only reaches the screen if something asks for a
//! repaint. These decide whether this frame is one of those, and are
//! deliberately conservative: notifying every poll would repaint thirty
//! times a second while an agent works.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::ClipboardItem;
use gpui_kit::Entity;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts;
use crate::panel_session;
use crate::working_indicator;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue;

/// Starts the window's repaint poll, which runs for the window's lifetime.
///
/// Two jobs the PTY reader thread cannot do itself, because it is not the
/// main thread: drain OSC 52 clipboard writes onto the pasteboard (see
/// `clipboard_writes`), and ask for a repaint when the terminal grid has
/// changed. Without the second the grid only updates on an unrelated UI
/// event - a keystroke, a mouse move - so output looks stalled after
/// pressing Enter.
pub(super) fn spawn_repaint_poll(view: Entity<WorkspaceWindow>,
                                 clipboard_writes: Arc<Mutex<Vec<String>>>,
                                 exited_sessions: Arc<Mutex<Vec<Uuid>>>, cx: &mut App) {
    cx.spawn(async move |cx| {
          loop {
              cx.background_executor()
                .timer(consts::REPAINT_POLL_INTERVAL)
                .await;
              let texts = std::mem::take(&mut *clipboard_writes.lock());
              let exited = std::mem::take(&mut *exited_sessions.lock());
              for text in texts {
                  cx.update(|app| {
                        app.write_to_clipboard(ClipboardItem::new_string(text));
                    });
              }
              cx.update(|app| {
                    view.update(app, |view, cx| view.repaint_poll_tick(&exited, cx));
                });
          }
      })
      .detach();
}

impl WorkspaceWindow {
    /// One tick of the poll, with the ids of any sessions whose process
    /// exited since the last one.
    fn repaint_poll_tick(&mut self, exited: &[Uuid], cx: &mut gpui_kit::Context<Self>) {
        // A shell companion whose process exited has nothing left to show,
        // so close it rather than leaving a dead pane that looks hung.
        for id in exited {
            // Before the removal: a terminal opened to hand the user an
            // agent's own MCP flow is how that section finds out anything
            // changed, and after `remove_agent` there is nothing left to ask.
            self.finish_mcp_handover(*id);
            self.remove_agent(*id, cx);
            cx.notify();
        }
        // Messages arrive from the MCP server on another thread; this poll
        // is where an agent going idle is noticed.
        self.deliver_inbox_nudges(cx);
        self.raise_awaiting_notifications(cx);
        self.raise_mcp_failure_notification(cx);
        // Before the repaint checks below, so an agent started here has its
        // slot in place when they run.
        let activated = self.activate_messaged_agents(cx);
        let grid_dirty = self.selected_agent
                             .and_then(|id| self.sessions.get(&id))
                             .and_then(|session| session.lock().grid())
                             .is_some_and(|grid| grid.lock().take_dirty());
        // Every agent's taps, not just the selected one's: an agent working
        // in an unselected pane is the case this feature exists for.
        let pull_requests_recorded = self.drain_pull_requests(cx);
        let prompts_completed = self.drain_prompt_results();
        let panel_states_moved = self.sync_panel_agent_states();
        let prompts_sent = self.deliver_waiting_prompts();
        let panel_dirty = self.panel_needs_repaint();
        // Both land from `spawn_blocking` with no context to notify from, so
        // without this a fetched pull request state drew only when something
        // unrelated happened to repaint the window - and on a workspace with
        // nothing running, that could be never.
        let forge_probed = self.forge_status.take_changed();
        let pull_request_states_changed = self.pull_request_states.take_changed();
        // The git panel's three off-main-thread sources, all draining here
        // for the same reason: none of them has a GPUI context, so each only
        // leaves something behind for this tick to act on. A staging
        // operation reports into a slot, a commit into another, and a
        // working-tree watch flips a flag.
        let git_actions_landed = self.drain_git_actions();
        let git_commits_landed = self.drain_git_commits();
        let git_watches_fired = self.drain_git_watches();
        let git_reads_landed = self.git_panel_needs_repaint();
        let spinner_dirty = self.spinner_repaint_due();
        // Runs `ps` on its own much slower cadence, and only while a
        // processes section is expanded on the shown agent - see
        // `workspace_window::processes`.
        let processes_sampled = self.process_sampling_tick();
        // Runs an agent's own MCP list command, off any cadence at all: on
        // first becoming visible, on refresh, and when a delegated terminal
        // exits. Lands here because `spawn_blocking` has no context to
        // notify from - see `workspace_window::mcp_panel::probe`.
        let mcp_probed = self.mcp_probe_tick(cx);
        if grid_dirty
           || panel_states_moved
           || panel_dirty
           || spinner_dirty
           || activated
           || processes_sampled
           || mcp_probed
           || prompts_completed
           || prompts_sent
           || pull_requests_recorded
           || forge_probed
           || pull_request_states_changed
           || git_actions_landed
           || git_commits_landed
           || git_watches_fired
           || git_reads_landed
        {
            cx.notify();
        }
        self.refresh_agents_menu(cx);
    }

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

    /// Applies the results of prompts that finished since the last poll to
    /// their queues, and says whether any did.
    ///
    /// Separate from [`Self::panel_needs_repaint`] because it changes state:
    /// that one is asked whether to draw, this one is what makes the answer
    /// yes.
    fn drain_prompt_results(&mut self) -> bool {
        let prompt_results = std::mem::take(&mut *self.panel_prompt_results.lock());
        let drained = !prompt_results.is_empty();
        for (id, prompt_id, result) in prompt_results {
            if let Some(queue) = self.panel_prompt_queues.get_mut(&id) {
                // A prompt the user deleted while it was in flight is
                // simply not there any more; `complete` ignores it.
                prompt_queue::complete(queue, prompt_id, result.is_ok());
            }
        }
        drained
    }

    /// Writes each ready panel session's lifecycle back to the store, so the
    /// sidebar and dashboard show what the ACP session is actually doing,
    /// and says whether any of them moved.
    ///
    /// Reporting it matters because this is the only thing that notices an
    /// *unselected* agent's turn ending: `panel_needs_repaint` consults the
    /// selected slot alone, by design - you cannot see an unselected
    /// panel - but the sidebar's state dot for that agent is on screen. Until
    /// this was polled, the transition reached the store and stopped there,
    /// and the dot caught up whenever something unrelated repainted the
    /// window.
    fn sync_panel_agent_states(&mut self) -> bool {
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
        let mut moved = false;
        {
            let mut store = self.store.lock();
            for (id, state) in panel_states {
                moved |= store.agent(id).is_some_and(|agent| agent.state != state);
                store.set_state(id, state);
            }
        }
        moved
    }

    /// Sends the next queued prompt to every agent that has one, and says
    /// whether any went out.
    fn deliver_waiting_prompts(&mut self) -> bool {
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
        let mut prompt_picked_up = false;
        for id in waiting {
            prompt_picked_up |= self.drain_panel_prompt(id);
        }
        prompt_picked_up
    }

    /// Whether anything the window draws outside the terminal grid has
    /// changed since the last poll: the selected agent's panel, or the diff
    /// stats any view may be showing.
    ///
    /// The two are combined here rather than inside
    /// [`Self::selected_panel_needs_repaint`] because `take_changed` clears
    /// the flag as it reads it. Asking for it down a path that can return
    /// early is how a landed diff stat gets consumed and thrown away -
    /// which it was, for any selected agent that had no panel session.
    pub(super) fn panel_needs_repaint(&mut self) -> bool {
        let stats_changed = self.diff_stats.take_changed();

        // Not `||`: the panel check has to run even when the stats already
        // decided the answer, because it clears its own flags too.
        self.selected_panel_needs_repaint() | stats_changed
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
    ///
    /// Nothing here is time-derived. The turn-in-progress row is an animated
    /// WebP that re-arms its own `request_animation_frame` (see
    /// `app_support::working_knot_animation`), so a turn that streams
    /// nothing for a while still animates without this poll waking the
    /// window. The braille spinner that did need driving from here is now
    /// only the dashboard's, and `spinner_repaint_due` drives that.
    fn selected_panel_needs_repaint(&mut self) -> bool {
        let Some(id) = self.selected_agent
        else {
            return false;
        };
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let (phase, events_arrived) = {
            let slot = slot.lock();
            let events_arrived = matches!(&*slot,
                                          panel_session::PanelSessionSlot::Ready(handle)
                                          if handle.take_dirty());
            (slot.phase(), events_arrived)
        };
        let phase_changed = self.panel_phases.insert(id, phase) != Some(phase);

        phase_changed || events_arrived
    }
}
