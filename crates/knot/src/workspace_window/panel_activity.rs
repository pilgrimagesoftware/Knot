//! The bridge from a Panel-mode agent's ACP session to its activity tracker.
//!
//! A panel session reports its lifecycle as *level*: `PanelState` holds
//! `pending_permission` and `turn_active`, and every repaint poll reads the
//! same answer until one of them changes. [`knot_activity::Tracker`] takes
//! *edges*: each `apply_acp_status` call is a reported transition, and
//! `ActivityState::apply_acp_status` pushes `Effect::AwaitingInput` every time
//! it is handed `Input`, not only the first. Converting one to the other is
//! this module's whole job, and the reason
//! [`WorkspaceWindow::sync_panel_agent_states`] calls the tracker only for
//! agents whose derived status differs from the one it last reported.
//!
//! Routing through the tracker rather than writing the store directly is what
//! makes the desktop notification and the idle delivery nudge reachable for a
//! Panel-mode agent: both are tracker effects, and a direct write emits
//! neither. See `openspec/specs/activity-detection/spec.md`, "ACP updates
//! drive status for Panel-mode agents".

use std::sync::atomic::Ordering;

use gpui_kit::Context;
use knot_activity::{EventSink, Tracker, TrackerConfig, tracking_for};
use knot_agents::AgentState;
use uuid::Uuid;

use crate::app_support::AwaitingInput;
use crate::panel_state::PanelState;
use crate::workspace_window::WorkspaceWindow;

/// The status an ACP session is reporting, and the text to put in front of
/// the user when that status is `Input`.
///
/// A pure function of the panel's state so the mapping is testable without a
/// window, a session or a runtime - the three things that make the caller
/// hard to reach from a test.
///
/// `tool_call_title` is the only human-readable field on a
/// `PermissionRequest`; `options` holds button labels, which say what the
/// user can answer rather than what is being asked. `None` carries no
/// message and the notification falls back to its default body, as
/// `desktop-notifications` requires.
pub(super) fn acp_status(state: &PanelState) -> (AgentState, Option<String>) {
    if let Some(request) = state.pending_permission.as_ref() {
        return (AgentState::Input, request.tool_call_title.clone());
    }
    if state.turn_active {
        return (AgentState::Running, None);
    }
    (AgentState::Idle, None)
}

/// One agent's reported ACP status and the attention message that goes with
/// it.
pub(super) type AcpStatus = (Uuid, (AgentState, Option<String>));

/// The subset of `derived` that is an actual transition: an agent whose
/// reported status differs from the one `current` last reported for it.
///
/// The level-to-edge conversion, kept as a pure function because the
/// property worth testing is what happens across *ticks* - the same pending
/// permission read thirty times reports once - and that is invisible in a
/// single call to the caller.
///
/// `current` is the window's own record of what it last handed the tracker,
/// never the agent store. The store is written by the tracker's sink a
/// channel hop later, so a tick that read it could see the previous status
/// and report the same permission request a second time - two notifications
/// for one prompt. An agent with no record yet is reported: it has never
/// been handed anything, so every status is a transition.
pub(super) fn transitions<F>(derived: Vec<AcpStatus>, current: F) -> Vec<AcpStatus>
    where F: Fn(Uuid) -> Option<AgentState> {
    derived.into_iter()
           .filter(|(id, (state, _))| current(*id) != Some(*state))
           .collect()
}

impl WorkspaceWindow {
    /// Whether `id` has an activity tracker, spawning one if this is the
    /// first tick that found its session ready.
    ///
    /// Lazy rather than built with the session so this stays out of
    /// `panel::session`, and because a tracker for a session that never
    /// becomes ready would arm timers nothing feeds - the same reason
    /// `knot-mcp-tools` gives for gating its own.
    ///
    /// Returns `false` when no tracker could be spawned, which leaves the
    /// caller to write the store itself. `Tracker::spawn` calls
    /// `tokio::spawn`, so it panics without an entered runtime; the window
    /// holds one and enters it here, and the `try_current` check keeps a
    /// caller that somehow has neither from taking the process down.
    pub(super) fn ensure_panel_tracker(&mut self, id: Uuid, cx: &mut Context<Self>) -> bool {
        if self.panel_trackers.contains_key(&id) {
            return true;
        }
        let Some(agent_type) = self.store
                                   .lock()
                                   .agent(id)
                                   .map(|agent| agent.agent_type.clone())
        else {
            return false;
        };
        let _runtime_guard = self.runtime.enter();
        if tokio::runtime::Handle::try_current().is_err() {
            return false;
        }
        // Panel, not Terminal: `ACP_UPDATES` is what keeps the idle timer and
        // the input-protection guard off these transitions, which
        // `activity-detection` requires. `knot-mcp-tools` hardcodes
        // `Terminal` at its own call, correctly for the hook route it serves;
        // the two must not be merged.
        let tracking = tracking_for(&agent_type, knot_core::ViewMode::Panel);
        let store = std::sync::Arc::clone(&self.store);
        let landed = std::sync::Arc::clone(&self.panel_status_landed);
        let queue = cx.has_global::<AwaitingInput>()
                      .then(|| cx.global::<AwaitingInput>().0.clone());
        let sink = EventSink { on_status:
                                   Some(Box::new(move |event: knot_activity::StatusEvent| {
                                            store.lock().set_state(id, event.status);
                                            // The write is what the dot draws
                                            // from, and it happens here, on
                                            // the tracker's task, with no
                                            // context to notify from. The
                                            // flag is how it reaches a frame.
                                            landed.store(true, Ordering::Release);
                                        })),
                               on_awaiting_input: queue.map(|queue| {
                                                           Box::new(move |message| {
                                                               queue.lock().push((id, message));
                                                           })
                                                           as Box<dyn FnMut(Option<String>) + Send>
                                                       }),
                               ..Default::default() };
        let tracker = Tracker::spawn(TrackerConfig::for_agent_type(agent_type), tracking, sink);
        self.panel_trackers.insert(id, tracker);
        true
    }
}

#[cfg(test)]
mod tests;
