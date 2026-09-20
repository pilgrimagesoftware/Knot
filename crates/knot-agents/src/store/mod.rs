use std::collections::BTreeMap;

use knot_core::{ActivationMode, Workspace};
use uuid::Uuid;

use crate::agent::{Agent, AgentState};

mod bench;
mod edit;
mod helpers;
mod lifecycle;
mod ordering;
mod panels;
mod persistence;
mod workspace;

#[cfg(test)]
mod tests;

const DEFAULT_WORKSPACE_NAME: &str = "Knot";
const DEFAULT_WORKSPACE_COLOR: &str = "#1B4FB2";

/// Fields a caller may override when creating an agent; everything else
/// (id, runtime state) is derived. `folder` is passed separately to
/// `AgentStore::create` since it is the one field every creation supplies.
#[derive(Debug, Clone, Default)]
pub struct CreateOptions {
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub agent_type: Option<String>,
    pub shell_command: Option<String>,
    pub persona_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub is_companion: bool,
    pub insert_after: Option<Uuid>,
    /// Defaults to `Passive` - deliberately not the load default a record
    /// with no stored mode gets (`Active`, see `knot_core::SavedAgent`).
    pub activation_mode: ActivationMode,
}

/// Fields an edit may change. `name`/`avatar` always apply and never trigger
/// a restart; `folder`/`agent_type`/persona changes do.
#[derive(Debug, Clone, Default)]
pub struct EditRequest {
    pub name: String,
    pub avatar: String,
    pub folder: Option<String>,
    pub agent_type: Option<String>,
    pub persona_id: Option<Uuid>,
    pub persona_changed: bool,
    pub relocate_companions: bool,
    /// Applied verbatim; changing it never triggers a restart, per
    /// `agent-lifecycle`'s "Activation mode" requirement.
    pub activation_mode: ActivationMode,
}

/// An agent removed by [`AgentStore::remove`], in cascade order (companions
/// before their owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedAgent {
    pub id: Uuid,
    /// Whether the caller owes MCP an unregister call for this id.
    pub was_registered: bool,
}

/// Owns the agent list and the workspaces that place them, implementing
/// every operation `openspec/specs/agent-lifecycle/spec.md` names.
#[derive(Debug, Clone, Default)]
pub struct AgentStore {
    agents: Vec<Agent>,
    workspaces: Vec<Workspace>,
    current_workspace_id: Option<Uuid>,
}

impl AgentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn saved_workspaces(&self) -> Vec<Workspace> {
        self.workspaces.clone()
    }

    pub fn agent(&self, id: Uuid) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub(super) fn agent_mut(&mut self, id: Uuid) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Mark whether this agent may start in this run. Set by selection and
    /// by `create` for an `Active` agent; cleared by deactivation. Runtime
    /// only - never persisted.
    pub fn set_activated(&mut self, id: Uuid, activated: bool) {
        if let Some(agent) = self.agent_mut(id) {
            agent.activated = activated;
        }
    }

    pub fn set_registered(&mut self, id: Uuid, registered: bool) {
        if let Some(agent) = self.agent_mut(id) {
            agent.is_registered = registered;
        }
    }

    pub fn set_session_id(&mut self, id: Uuid, session_id: String) {
        if let Some(agent) = self.agent_mut(id) {
            agent.session_id = Some(session_id);
        }
    }

    pub fn set_acp_session_id(&mut self, id: Uuid, session_id: String) {
        if let Some(agent) = self.agent_mut(id) {
            agent.acp_session_id = Some(session_id);
        }
    }

    pub fn apply_acp_session_outcomes(&mut self, outcomes: &BTreeMap<Uuid, Option<String>>) {
        for agent in &mut self.agents {
            if let Some(outcome) = outcomes.get(&agent.id) {
                agent.acp_session_id = outcome.clone();
            }
        }
    }

    pub fn set_state(&mut self, id: Uuid, state: AgentState) {
        if let Some(agent) = self.agent_mut(id) {
            agent.state = state;
        }
    }

    pub fn update_metadata(&mut self, id: Uuid, metadata: BTreeMap<String, String>) {
        if let Some(agent) = self.agent_mut(id) {
            agent.metadata.extend(metadata);
        }
    }

    pub fn set_status_text(&mut self, id: Uuid, status: String) {
        if let Some(agent) = self.agent_mut(id) {
            agent.status_text = status;
        }
    }

    pub fn set_terminal_title(&mut self, id: Uuid, title: String) {
        if let Some(agent) = self.agent_mut(id) {
            agent.terminal_title = title;
        }
    }
}
