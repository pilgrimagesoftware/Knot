use std::collections::BTreeMap;

use knot_core::{
    ActivationMode, Capabilities, CostTier, SavedPullRequest, Workspace, WorkspaceUiState,
};
use uuid::Uuid;

use crate::agent::{Agent, AgentState};

mod bench;
mod edit;
mod helpers;
mod lifecycle;
mod ordering;
mod panels;
mod persistence;
mod pull_requests;

pub use persistence::AdoptedCounts;
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
    pub name:            Option<String>,
    pub avatar:          Option<String>,
    pub agent_type:      Option<String>,
    pub shell_command:   Option<String>,
    pub persona_id:      Option<Uuid>,
    pub created_by:      Option<Uuid>,
    pub is_companion:    bool,
    pub insert_after:    Option<Uuid>,
    /// Workspace to place the new agent in. Takes priority over inferring one
    /// from `created_by`/`insert_after`. A caller that knows which workspace
    /// it means must say so explicitly rather than relying on whichever
    /// workspace happens to be "current" - that's ambient global state a
    /// concurrent window or MCP call can change out from under it.
    pub workspace_id:    Option<Uuid>,
    /// Defaults to `Passive` - deliberately not the load default a record
    /// with no stored mode gets (`Active`, see `knot_core::SavedAgent`).
    pub activation_mode: ActivationMode,
    /// Registry metadata. All three default to "nothing declared", which is
    /// what an agent created without them should read as.
    pub description:     String,
    pub capabilities:    Capabilities,
    pub cost_tier:       CostTier,
}

/// Fields an edit may change. `name`/`avatar` always apply and never trigger
/// a restart; `folder`/`agent_type`/persona changes do.
#[derive(Debug, Clone, Default)]
pub struct EditRequest {
    pub name:                String,
    pub avatar:              String,
    pub folder:              Option<String>,
    pub agent_type:          Option<String>,
    pub persona_id:          Option<Uuid>,
    pub persona_changed:     bool,
    pub relocate_companions: bool,
    /// Applied verbatim; changing it never triggers a restart, per
    /// `agent-lifecycle`'s "Activation mode" requirement.
    pub activation_mode:     ActivationMode,
    /// Registry metadata, applied verbatim. Like `activation_mode`, none of
    /// it triggers a restart: re-tagging an agent says nothing about the
    /// session it already has, and interrupting one to record a label would
    /// throw away the work it is doing.
    pub description:         String,
    pub capabilities:        Capabilities,
    pub cost_tier:           CostTier,
}

/// An agent removed by [`AgentStore::remove`], in cascade order (companions
/// before their owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedAgent {
    pub id:             Uuid,
    /// Whether the caller owes MCP an unregister call for this id.
    pub was_registered: bool,
}

/// Owns the agent list and the workspaces that place them, implementing
/// every operation `openspec/specs/agent-lifecycle/spec.md` names.
#[derive(Debug, Clone, Default)]
pub struct AgentStore {
    agents:               Vec<Agent>,
    workspaces:           Vec<Workspace>,
    /// How each workspace's window is arranged, keyed by workspace id.
    ///
    /// A second per-key map on a struct that already has several, which the
    /// parallel-maps rule warns about. Accepted here because the two have
    /// genuinely different lifetimes - a workspace's configuration outlives
    /// any window, and its arrangement is meaningless without one - and
    /// because only this one is rewritten by a pointer drag. The pruning that
    /// keeps them from disagreeing is in `remove_workspace` and, for anything
    /// that removes a workspace by another path, on load in `knot-core`. See
    /// `openspec/specs/settings-persistence/spec.md`.
    workspace_ui:         BTreeMap<Uuid, WorkspaceUiState>,
    current_workspace_id: Option<Uuid>,
    /// The pull requests Knot has seen in these agents' output. Held here
    /// rather than only in the settings document so that removing an agent
    /// or a workspace can take its records with it, in the one place that
    /// knows either is going away. See [`pull_requests`].
    pull_requests:        Vec<SavedPullRequest>,
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

    /// The whole UI-state map, for persisting it.
    pub fn saved_workspace_ui(&self) -> BTreeMap<Uuid, WorkspaceUiState> {
        self.workspace_ui.clone()
    }

    pub fn agent(&self, id: Uuid) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    pub(super) fn agent_mut(&mut self, id: Uuid) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Apply `f` to the agent with `id`, if the store still holds one.
    ///
    /// Every one-line setter below is this and nothing else. An unknown id
    /// is not an error: an agent can be removed while a message about it is
    /// still in flight, and the update is then simply dropped.
    fn update(&mut self, id: Uuid, f: impl FnOnce(&mut Agent)) {
        if let Some(agent) = self.agent_mut(id) {
            f(agent);
        }
    }

    /// Mark whether this agent may start in this run. Set by selection and
    /// by `create` for an `Active` agent; cleared by deactivation. Runtime
    /// only - never persisted.
    pub fn set_activated(&mut self, id: Uuid, activated: bool) {
        self.update(id, |agent| agent.activated = activated);
    }

    pub fn set_registered(&mut self, id: Uuid, registered: bool) {
        self.update(id, |agent| agent.is_registered = registered);
    }

    pub fn set_session_id(&mut self, id: Uuid, session_id: String) {
        self.update(id, |agent| agent.session_id = Some(session_id));
    }

    pub fn set_acp_session_id(&mut self, id: Uuid, session_id: String) {
        self.update(id, |agent| agent.acp_session_id = Some(session_id));
    }

    pub fn apply_acp_session_outcomes(&mut self, outcomes: &BTreeMap<Uuid, Option<String>>) {
        for agent in &mut self.agents {
            if let Some(outcome) = outcomes.get(&agent.id) {
                agent.acp_session_id = outcome.clone();
            }
        }
    }

    pub fn set_state(&mut self, id: Uuid, state: AgentState) {
        self.update(id, |agent| agent.state = state);
    }

    pub fn update_metadata(&mut self, id: Uuid, metadata: BTreeMap<String, String>) {
        self.update(id, |agent| agent.metadata.extend(metadata));
    }

    pub fn set_status_text(&mut self, id: Uuid, status: String) {
        self.update(id, |agent| agent.status_text = status);
    }

    pub fn set_terminal_title(&mut self, id: Uuid, title: String) {
        self.update(id, |agent| agent.terminal_title = title);
    }
}
