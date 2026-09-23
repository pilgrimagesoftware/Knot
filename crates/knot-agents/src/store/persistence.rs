use std::collections::BTreeMap;

use knot_core::SavedAgent;
use uuid::Uuid;

use super::AgentStore;
use crate::convert::from_saved;

/// How many records [`AgentStore::adopt_saved`] took in, as opposed to
/// recognised and left alone.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AdoptedCounts {
    pub agents:     usize,
    pub workspaces: usize,
}

impl AgentStore {
    pub fn from_saved(saved_agents: &[SavedAgent], workspaces: Vec<knot_core::Workspace>) -> Self {
        let agents = saved_agents.iter().map(from_saved).collect::<Vec<_>>();
        let use_saved_workspaces = agents.is_empty() || !workspaces.is_empty();
        let workspaces = if use_saved_workspaces {
            workspaces
        }
        else {
            vec![super::helpers::default_workspace(agents.iter().map(|agent| agent.id).collect())]
        };
        let current_workspace_id = workspaces.first().map(|workspace| workspace.id);
        let mut store = Self { agents,
                               workspaces,
                               // Filled from its own document by the caller,
                               // the same as `pull_requests`: which agents
                               // this store holds says nothing about how any
                               // window was arranged.
                               workspace_ui: BTreeMap::new(),
                               current_workspace_id,
                               // Filled from its own document by the caller,
                               // which is what loads it: rebuilding the
                               // roster says nothing about what Knot has
                               // seen.
                               pull_requests: Vec::new() };
        // A workspace invented here has no saved arrangement to restore, so
        // the first agent is what its window shows.
        if !use_saved_workspaces
           && let (Some(id), Some(first)) =
               (store.current_workspace_id, store.agents.first().map(|agent| agent.id))
        {
            store.set_workspace_active_agents(id, vec![first]);
        }
        store
    }

    /// Take in agents and workspaces that already exist elsewhere, keeping
    /// their identifiers, and report how many of each were new.
    ///
    /// This is what an import needs and [`Self::from_saved`] is not: the store
    /// is already live, holding agents with running sessions and state that a
    /// rebuild would discard. A record whose id the store already holds is
    /// left exactly as it is - the live copy wins over the incoming one, which
    /// is also what makes calling this twice harmless.
    ///
    /// Without it an import wrote only to the settings document, while the
    /// store went on holding what it loaded at startup - so imported
    /// workspaces were invisible to every open window, and the next
    /// store-to-settings write erased them.
    pub fn adopt_saved(&mut self, agents: &[SavedAgent], workspaces: &[knot_core::Workspace])
                       -> AdoptedCounts {
        let mut adopted = AdoptedCounts::default();

        for saved in agents {
            if self.agents.iter().any(|agent| agent.id == saved.id) {
                continue;
            }
            self.agents.push(from_saved(saved));
            adopted.agents += 1;
        }

        for workspace in workspaces {
            if self.workspaces.iter().any(|held| held.id == workspace.id) {
                continue;
            }
            self.workspaces.push(workspace.clone());
            adopted.workspaces += 1;
        }

        // A store that held nothing before has no current workspace, and a
        // window with none selected shows an empty list however many were
        // just adopted.
        if self.current_workspace_id.is_none() {
            self.current_workspace_id = self.workspaces.first().map(|workspace| workspace.id);
        }

        adopted
    }

    pub fn saved_agents(&self, remember_conversation: bool) -> Vec<SavedAgent> {
        self.agents
            .iter()
            .map(|agent| crate::convert::to_saved(agent, remember_conversation))
            .collect()
    }

    pub fn resolve_resume_sessions<F>(&mut self, persisted: &BTreeMap<Uuid, String>,
                                      mut resolver: F)
        where F: FnMut(&str, &str) -> Option<String> {
        for agent in &mut self.agents {
            if agent.resume_session_id.is_none() {
                agent.resume_session_id =
                    persisted.get(&agent.id)
                             .cloned()
                             .or_else(|| resolver(&agent.folder, &agent.agent_type));
            }
        }
    }
}
