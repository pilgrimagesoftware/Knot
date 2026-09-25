//! The ACP feed's writer into [`SubagentRegistry`].
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "Subagents reach Knot
//! by the agent's own reporting".
//!
//! A Panel-mode agent's session events already stream past
//! [`crate::panel_session`]'s drain on a tokio task. This watches that stream
//! for delegations and writes them to the shared registry; the hook route is
//! the other writer, and neither knows about the other.
//!
//! Here rather than inside the drain loop because the drain's job is to fold
//! an event into `PanelState` and flip a dirty bit, and recognition is neither.
//! Keeping it out also means the sink is constructible in a test without an
//! adapter subprocess.
//!
//! A `None` sink is the ordinary state for most agents: [`SubagentSink::new`]
//! answers `None` when the agent's type has no recognizer, which is what
//! `SubagentReporting::None` on the roster describes. Nothing downstream then
//! pays for the feed at all.

use std::sync::Arc;
use std::time::Instant;

use knot_acp::{SessionEvent, SessionUpdate, ToolCallContent};
use knot_subagents::recognize::{Recognizer, ToolCallReport, for_agent_type};
use knot_subagents::registry::SubagentRegistry;
use parking_lot::Mutex;
use uuid::Uuid;

#[cfg(test)]
mod tests;

/// Watches one agent's session events for subagent lifecycle reports.
#[derive(Clone)]
pub(crate) struct SubagentSink {
    agent:      Uuid,
    recognizer: &'static dyn Recognizer,
    registry:   Arc<Mutex<SubagentRegistry>>,
}

impl SubagentSink {
    /// A sink for `agent`, or `None` if its type reports no subagents.
    pub(crate) fn new(agent: Uuid, agent_type: &str, registry: Arc<Mutex<SubagentRegistry>>)
                      -> Option<Self> {
        Some(Self { agent,
                    recognizer: for_agent_type(agent_type)?,
                    registry })
    }

    /// Records whatever this event says about the agent's subagents.
    ///
    /// Takes the event by reference so the drain can still hand it to
    /// `PanelState::apply` by value afterwards; nothing here mutates it.
    pub(crate) fn observe(&self, event: &SessionEvent) {
        match event {
            SessionEvent::Update(update) => self.observe_update(update),
            // The session is gone, so its records describe a turn that can no
            // longer be running. Same reasoning as a turn ending.
            SessionEvent::Ended(_) => self.registry.lock().clear(self.agent),
            SessionEvent::PermissionRequest(_) => {}
        }
    }

    fn observe_update(&self, update: &SessionUpdate) {
        let report = match update {
            SessionUpdate::ToolCallStart { tool_call_id,
                                           status,
                                           content,
                                           raw_input,
                                           meta,
                                           .. } => ToolCallReport { id:          tool_call_id,
                                                                    meta:        meta.as_ref(),
                                                                    raw_input:   raw_input.as_ref(),
                                                                    status:      Some(status),
                                                                    result_text:
                                                                        first_text(content), },
            SessionUpdate::ToolCallUpdate { tool_call_id,
                                            status,
                                            content,
                                            raw_input,
                                            meta,
                                            .. } => {
                ToolCallReport { id:          tool_call_id,
                                 meta:        meta.as_ref(),
                                 raw_input:   raw_input.as_ref(),
                                 status:      status.as_deref(),
                                 result_text: first_text(content), }
            }
            // A turn ending discards the turn's records, per the spec. The
            // ACP signal; the hook feed's equivalent is the agent going idle.
            SessionUpdate::TurnEnd { .. } => {
                self.registry.lock().clear(self.agent);
                return;
            }
            _ => return,
        };

        let Some(event) = self.recognizer.recognize(&report)
        else {
            return;
        };

        self.registry
            .lock()
            .apply(self.agent, event, Instant::now());
    }
}

/// A tool call's first text block, which is where a failed delegation's
/// reason arrives - there is no error field of its own on this path.
fn first_text(content: &[ToolCallContent]) -> Option<&str> {
    content.iter().find_map(|item| match item {
                      ToolCallContent::Text(text) if !text.trim().is_empty() => Some(text.as_str()),
                      _ => None,
                  })
}
