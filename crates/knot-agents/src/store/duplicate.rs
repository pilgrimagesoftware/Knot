//! Duplicating an agent: a new agent set up exactly like an existing one,
//! with none of its conversation or run state.

use uuid::Uuid;

use super::{AgentStore, CreateOptions};
use crate::agent::Agent;

impl AgentStore {
    /// Creates a copy of `source` directly after it, named with the next
    /// free number off its name, and returns the copy's id. `None` when
    /// `source` does not exist.
    ///
    /// Every field of [`Agent`] is named below, so adding one fails to
    /// compile here until someone decides whether a duplicate carries it.
    /// That is the point: the copy used to be a hand-listed
    /// `CreateOptions`, and the registry metadata (description,
    /// capabilities, cost tier), the activation mode and the session
    /// setup were each added to `Agent` after it and silently left out
    /// (#556).
    pub fn duplicate(&mut self, source: Uuid) -> Option<Uuid> {
        let Agent { // The new agent's own identity.
                    id: _,
                    name,
                    // Set up the same way.
                    avatar,
                    folder,
                    agent_type,
                    shell_command,
                    persona_id,
                    activation_mode,
                    description,
                    capabilities,
                    cost_tier,
                    session_config,
                    startup_prompt,
                    // Follows from `agent_type` in `create`.
                    view_mode: _,
                    // Not copied: only an owner can be duplicated (companions
                    // follow their owner), and the copy belongs to no one.
                    created_by: _,
                    is_companion: _,
                    // Runtime and conversation state: a duplicate starts
                    // fresh rather than sharing the source's session.
                    activated: _,
                    state: _,
                    idle_since: _,
                    status_text: _,
                    is_registered: _,
                    is_pending_start: _,
                    terminal_title: _,
                    restart_token: _,
                    session_id: _,
                    resume_session_id: _,
                    fork_session: _,
                    acp_session_id: _,
                    metadata: _,
                    markdown_file: _,
                    markdown_maximized: _,
                    markdown_history: _,
                    mermaid_source: _,
                    mermaid_title: _, } = self.agent(source)?.clone();

        let name =
            crate::duplicate_name(&name, self.agents.iter().map(|agent| agent.name.as_str()));
        let id = self.create(folder,
                             CreateOptions { name: Some(name),
                                             avatar: Some(avatar),
                                             agent_type: Some(agent_type),
                                             shell_command,
                                             persona_id,
                                             insert_after: Some(source),
                                             activation_mode,
                                             description,
                                             capabilities,
                                             cost_tier,
                                             startup_prompt,
                                             ..Default::default() });
        // `CreateOptions` has no session setup: a new agent has made no
        // choices yet. A duplicate has made the source's.
        if let Some(copy) = self.agents.iter_mut().find(|agent| agent.id == id) {
            copy.session_config = session_config;
        }
        Some(id)
    }
}
