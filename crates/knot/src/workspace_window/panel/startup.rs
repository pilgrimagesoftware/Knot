//! The startup prompt's two window-side steps: what a fresh panel session is
//! handed to expand, and moving the expanded prompt into the agent's queue.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`, "Startup prompt
//! follows the initialization prompt".
//!
//! The expansion itself happens in `panel_session::connect_into`, on the
//! runtime, because reading `{{branch}}` runs a subprocess. What comes back
//! is taken here, from `repaint_poll_tick`'s `deliver_waiting_prompts`, so
//! it rides the one path that is already polled for every agent.

use gpui_kit::App;
use knot_agents::Agent;

use crate::panel_session::{PanelSessionSlot, StartupRequest};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::{PromptOrigin, QueuedPanelPrompt};

impl WorkspaceWindow {
    /// What `agent`'s fresh session needs to expand its startup prompt, or
    /// `None` when it has none - including a library reference to a prompt
    /// no longer in the library, which resolves to nothing.
    ///
    /// A settings read and a store read, no I/O: the branch is read later,
    /// off this thread.
    pub(in crate::workspace_window) fn startup_request(&self, agent: &Agent, cx: &App)
                                                       -> Option<StartupRequest> {
        let text = {
            let settings = crate::settings_global::read(cx);
            knot_agent_launch::resolve_startup_prompt(agent.startup_prompt.as_ref(),
                                                      &settings.prompts)?
        };
        Some(StartupRequest { text,
                              context: self.prompt_context_source(agent) })
    }

    /// What `agent` contributes to the context a prompt's variables expand
    /// with - everything but the branch and the date, which
    /// `knot_agent_launch::read_context` reads off this thread.
    pub(in crate::workspace_window) fn prompt_context_source(
        &self, agent: &Agent)
        -> knot_agent_launch::ContextSource {
        let workspace = self.store
                            .lock()
                            .workspaces()
                            .iter()
                            .find(|workspace| workspace.agent_ids.contains(&agent.id))
                            .map(|workspace| workspace.name.clone());
        knot_agent_launch::ContextSource { agent_name: agent.name.clone(),
                                           agent_id: agent.id.to_string(),
                                           agent_type: agent.agent_type.clone(),
                                           folder: agent.folder.clone(),
                                           workspace }
    }

    /// Moves every `Ready` session's expanded startup prompt onto its agent's
    /// queue, behind the registration turn already running there, and says
    /// whether any moved.
    ///
    /// Each handle gives its prompt up once, so a session is queued at most
    /// one startup prompt however many ticks see it `Ready`.
    pub(in crate::workspace_window) fn queue_startup_prompts(&mut self) -> bool {
        let mut queued = false;
        for (id, slot) in &self.panel_sessions {
            let text = match &mut *slot.lock() {
                PanelSessionSlot::Ready(handle) => handle.take_startup_prompt(),
                PanelSessionSlot::Connecting(_) | PanelSessionSlot::Failed(_) => None,
            };
            if let Some(text) = text {
                self.panel_prompt_queues
                    .entry(*id)
                    .or_default()
                    .push(QueuedPanelPrompt::new(text, PromptOrigin::Startup));
                queued = true;
            }
        }
        queued
    }
}

#[cfg(test)]
mod tests;
