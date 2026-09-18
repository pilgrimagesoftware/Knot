use std::collections::BTreeMap;

use uuid::Uuid;

use super::{AgentStore, CreateOptions, RemovedAgent};
use crate::agent::{Agent, AgentState, view_mode_for};
use crate::error::{AgentError, Result};

impl AgentStore {
    pub fn create(&mut self, folder: impl Into<String>, opts: CreateOptions) -> Uuid {
        let folder = folder.into();
        let name = opts.name
                       .unwrap_or_else(|| super::helpers::last_path_component(&folder));
        let agent_type = opts.agent_type.unwrap_or_else(|| "claude".to_string());
        let agent = Agent { id: Uuid::new_v4(),
                            name,
                            avatar: opts.avatar.unwrap_or_default(),
                            folder,
                            view_mode: view_mode_for(&agent_type),
                            agent_type,
                            created_by: opts.created_by,
                            is_companion: opts.is_companion,
                            shell_command: opts.shell_command,
                            persona_id: opts.persona_id,
                            state: AgentState::Idle,
                            status_text: String::new(),
                            is_registered: false,
                            is_pending_start: false,
                            terminal_title: String::new(),
                            restart_token: Uuid::new_v4(),
                            session_id: None,
                            resume_session_id: None,
                            fork_session: false,
                            acp_session_id: None,
                            metadata: BTreeMap::new(),
                            markdown_file: None,
                            markdown_maximized: false,
                            markdown_history: Vec::new(),
                            mermaid_source: None,
                            mermaid_title: None };
        let id = agent.id;
        match opts.insert_after
                  .and_then(|sibling| self.agents.iter().position(|a| a.id == sibling))
        {
            Some(index) => self.agents.insert(index + 1, agent),
            None => self.agents.push(agent),
        }
        let source = opts.created_by.or(opts.insert_after);
        let workspace_id = source.and_then(|source| self.workspace_of(source))
                                 .unwrap_or_else(|| self.ensure_current_workspace());
        if let Some(workspace) = self.workspaces
                                     .iter_mut()
                                     .find(|workspace| workspace.id == workspace_id)
        {
            match opts.insert_after
                      .and_then(|sibling| workspace.agent_ids.iter().position(|id| *id == sibling))
            {
                Some(index) => workspace.agent_ids.insert(index + 1, id),
                None => workspace.agent_ids.push(id),
            }
            if workspace.active_agent_ids.is_empty() {
                workspace.active_agent_ids = vec![id];
            }
        }
        id
    }

    pub fn create_shell_companion(&mut self, owner: Uuid) -> Result<Uuid> {
        let owner_agent = self.agent(owner).ok_or(AgentError::NotFound(owner))?;
        if owner_agent.is_companion {
            return Err(AgentError::CompanionCannotOwn(owner));
        }
        Ok(self.create(owner_agent.folder.clone(),
                       CreateOptions { name: Some("Shell".to_string()),
                                       agent_type: Some("shell".to_string()),
                                       created_by: Some(owner),
                                       is_companion: true,
                                       insert_after: Some(owner),
                                       ..Default::default() }))
    }

    pub fn companions(&self, owner: Uuid) -> Vec<Uuid> {
        self.agents
            .iter()
            .filter(|agent| agent.created_by == Some(owner) && agent.is_companion)
            .map(|agent| agent.id)
            .collect()
    }

    pub fn remove(&mut self, id: Uuid) -> Vec<RemovedAgent> {
        let mut removed = Vec::new();
        for companion_id in self.companions(id) {
            removed.extend(self.remove(companion_id));
        }
        if let Some(position) = self.agents.iter().position(|agent| agent.id == id) {
            let was_registered = self.agents[position].is_registered;
            self.agents.remove(position);
            for workspace in &mut self.workspaces {
                workspace.agent_ids.retain(|agent_id| *agent_id != id);
                workspace.active_agent_ids
                         .retain(|agent_id| *agent_id != id);
            }
            removed.push(RemovedAgent { id, was_registered });
        }
        removed
    }

    fn recreate_terminal(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.restart_token = Uuid::new_v4();
        agent.state = AgentState::Idle;
        agent.is_registered = false;
        agent.terminal_title = String::new();
        Ok(())
    }

    pub fn restart(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.session_id = None;
        agent.acp_session_id = None;
        agent.resume_session_id = None;
        agent.fork_session = false;
        self.recreate_terminal(id)
    }

    pub fn resume_session(&mut self, id: Uuid, session_id: impl Into<String>) -> Result<()> {
        let session_id = session_id.into();
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.resume_session_id = Some(session_id.clone());
        agent.session_id = Some(session_id);
        agent.fork_session = false;
        self.recreate_terminal(id)
    }

    /// Points a freshly created agent at an existing session as a *fork*:
    /// it continues that conversation without the source giving it up.
    ///
    /// Sets the same fields as [`Self::resume_session`] plus the fork flag,
    /// and recreates no terminal - the agent has not been started yet, so
    /// there is nothing to recreate.
    pub fn fork_session(&mut self, id: Uuid, session_id: impl Into<String>) -> Result<()> {
        let session_id = session_id.into();
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.resume_session_id = Some(session_id.clone());
        agent.session_id = Some(session_id);
        agent.fork_session = true;
        Ok(())
    }
}
