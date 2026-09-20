use std::collections::BTreeMap;

use knot_core::SavedAgent;
use uuid::Uuid;

use super::AgentStore;
use crate::convert::from_saved;

impl AgentStore {
    pub fn from_saved(saved_agents: &[SavedAgent], workspaces: Vec<knot_core::Workspace>) -> Self {
        let agents = saved_agents.iter().map(from_saved).collect::<Vec<_>>();
        let workspaces = if agents.is_empty() || !workspaces.is_empty() {
            workspaces
        } else {
            vec![super::helpers::default_workspace(
                agents.iter().map(|agent| agent.id).collect(),
            )]
        };
        let current_workspace_id = workspaces.first().map(|workspace| workspace.id);
        Self {
            agents,
            workspaces,
            current_workspace_id,
        }
    }

    pub fn saved_agents(&self, remember_conversation: bool) -> Vec<SavedAgent> {
        self.agents
            .iter()
            .map(|agent| crate::convert::to_saved(agent, remember_conversation))
            .collect()
    }

    pub fn resolve_resume_sessions<F>(
        &mut self, persisted: &BTreeMap<Uuid, String>, mut resolver: F,
    ) where
        F: FnMut(&str, &str) -> Option<String>,
    {
        for agent in &mut self.agents {
            if agent.resume_session_id.is_none() {
                agent.resume_session_id = persisted
                    .get(&agent.id)
                    .cloned()
                    .or_else(|| resolver(&agent.folder, &agent.agent_type));
            }
        }
    }
}
