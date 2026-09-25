use std::collections::BTreeMap;

use knot_core::ActivationMode;
use uuid::Uuid;

use super::{AgentStore, CreateOptions, RemovedAgent};
use crate::agent::{Agent, AgentState, view_mode_for};
use crate::error::{AgentError, Result};

impl AgentStore {
    pub fn create(&mut self, folder: impl Into<String>, opts: CreateOptions) -> Uuid {
        let folder = folder.into();
        let name = opts.name
                       .unwrap_or_else(|| super::helpers::name_for_folder(&folder));
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
                            activation_mode: opts.activation_mode,
                            description: opts.description,
                            capabilities: opts.capabilities,
                            cost_tier: opts.cost_tier,
                            session_config: BTreeMap::new(),
                            // An `Active` agent is activated from birth, so
                            // it starts when its workspace next opens.
                            activated: opts.activation_mode == ActivationMode::Active,
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
        let workspace_id = opts.workspace_id
                               .or_else(|| source.and_then(|source| self.workspace_of(source)))
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
            // The first agent in a workspace is the one its window shows, so
            // an otherwise-empty layout gets it. Which agents are active is
            // arrangement, so it is set on the UI state rather than the
            // record.
            if self.workspace_ui(workspace_id).active_agent_ids.is_empty() {
                self.set_workspace_active_agents(workspace_id, vec![id]);
            }
        }
        id
    }

    /// A companion inherits its owner's activation mode, and starts right
    /// away if its owner is already running: it exists to sit beside that
    /// owner, and deactivation already takes companions with it. Given the
    /// default `Passive` instead, adding a shell to a running agent would
    /// produce a row that does nothing until it is clicked.
    pub fn create_shell_companion(&mut self, owner: Uuid) -> Result<Uuid> {
        let owner_agent = self.agent(owner).ok_or(AgentError::NotFound(owner))?;
        if owner_agent.is_companion {
            return Err(AgentError::CompanionCannotOwn(owner));
        }
        let folder = owner_agent.folder.clone();
        let activation_mode = owner_agent.activation_mode;
        let owner_activated = owner_agent.activated;
        let id = self.create(folder,
                             CreateOptions { name: Some("Shell".to_string()),
                                             agent_type: Some("shell".to_string()),
                                             created_by: Some(owner),
                                             is_companion: true,
                                             insert_after: Some(owner),
                                             activation_mode,
                                             ..Default::default() });
        self.set_activated(id, owner_activated);
        Ok(id)
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
            self.forget_agent_pull_requests(id);
            for workspace in &mut self.workspaces {
                workspace.agent_ids.retain(|agent_id| *agent_id != id);
            }
            // The parallel map has to lose the agent too, or a layout would
            // go on naming a pane for an agent that no longer exists.
            for ui in self.workspace_ui.values_mut() {
                ui.active_agent_ids.retain(|agent_id| *agent_id != id);
            }
            removed.push(RemovedAgent { id, was_registered });
        }
        removed
    }

    /// Marks every `Active` agent among `ids` activated, so opening a
    /// workspace starts its `active` agents and only those, per
    /// `agent-lifecycle`'s "Activation mode". Returns the ids it activated.
    ///
    /// This, not [`Self::create`]'s flag, is what survives a relaunch:
    /// `activated` is runtime-only and loads false for every agent, so the
    /// durable mode has to be consulted each time a workspace opens. A
    /// `Passive` agent is left alone, and so is one the user deactivated:
    /// deactivation lasts as long as the workspace stays open, and this
    /// runs only when it opens.
    pub fn activate_on_workspace_open(&mut self, ids: &[Uuid]) -> Vec<Uuid> {
        let active = ids.iter()
                        .copied()
                        .filter(|id| {
                            self.agent(*id).is_some_and(|agent| {
                                               agent.activation_mode == ActivationMode::Active
                                           })
                        })
                        .collect::<Vec<_>>();
        for id in &active {
            self.set_activated(*id, true);
        }
        active
    }

    /// Clears the `activated` flag on `id` and every companion it owns,
    /// returning them in cascade order - companions first, then the owner -
    /// so the caller can tear their sessions down in the same order
    /// [`Self::remove`] hands back removals.
    ///
    /// Touches nothing else: the agents stay in the list, in their order,
    /// in their workspaces, with their names, folders, personas and
    /// activation modes, per `agent-lifecycle`'s "Deactivating an agent".
    pub fn deactivate(&mut self, id: Uuid) -> Vec<Uuid> {
        let mut deactivated = Vec::new();
        for companion_id in self.companions(id) {
            deactivated.extend(self.deactivate(companion_id));
        }
        if let Some(agent) = self.agent_mut(id) {
            agent.activated = false;
            deactivated.push(id);
        }
        deactivated
    }

    fn recreate_terminal(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.restart_token = Uuid::new_v4();
        agent.state = AgentState::Idle;
        agent.is_registered = false;
        agent.terminal_title = String::new();
        Ok(())
    }

    /// Restarts `id` into a new conversation: every session id is cleared,
    /// so the relaunch starts a fresh session with the full initialization
    /// prompt. What a launch-affecting edit and Restart with New
    /// Conversation do, and what Restart does with
    /// `restore-conversation-on-launch` off (`agent-lifecycle`: "Restart").
    pub fn restart(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.session_id = None;
        agent.acp_session_id = None;
        agent.resume_session_id = None;
        agent.fork_session = false;
        self.recreate_terminal(id)
    }

    /// Restarts `id` back into the conversation it has: the session ids are
    /// kept, and the resume-session id is pointed at the session id, so a
    /// Panel-mode agent loads its ACP session again on relaunch. What
    /// Restart does with `restore-conversation-on-launch` on.
    ///
    /// The fork flag is cleared, as by [`Self::restart`]: a fork is an
    /// instruction for the agent's first launch, and one that can be
    /// restarted has already had it.
    pub fn restart_keeping_conversation(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.resume_session_id = agent.session_id.clone();
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
