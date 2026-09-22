//! The window's agent roster: selection, removal, deactivation, the bulk
//! actions the sidebar's background menu runs, and the persist that has to
//! follow any of them.
//!
//! Every mutation here goes through `AgentStore` and then
//! [`WorkspaceWindow::persist_agents`] - `AgentStore` only mutates memory,
//! so a roster change that skips the persist is undone by the next launch.

use uuid::Uuid;

use crate::workspace_window::PromptOrigin;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::workspace_agent_ids;

impl WorkspaceWindow {
    /// Removes `id` (and its companions) from the store, tears down their
    /// sessions, and persists the result.
    ///
    /// The persist is the point: `AgentStore::remove` only mutates memory,
    /// so without writing `saved_agents`/`saved_workspaces` back out the
    /// removal was undone by the next launch (or by any other window's
    /// persist), which is what "Remove Agent does nothing" looked like.
    /// Every other agent mutation (create, edit) already persists this way.
    pub(super) fn remove_agent(&mut self, id: Uuid) {
        self.remove_agent_unpersisted(id);
        self.persist_agents();
    }

    /// The removal itself, without writing settings back out.
    ///
    /// Split so a bulk close can remove every agent and persist once at the
    /// end rather than rewriting `saved_agents` per agent, without a second
    /// copy of the teardown cascade going out of step with this one.
    pub(super) fn remove_agent_unpersisted(&mut self, id: Uuid) {
        let removed = self.store.lock().remove(id);
        for removed_agent in removed {
            self.teardown_session(removed_agent.id);
            if self.selected_agent == Some(removed_agent.id) {
                self.selected_agent = None;
            }
        }
    }

    /// Makes `id` this window's selection and marks it activated, so the
    /// next `ensure_*` starts it. Selection is the universal "I want this
    /// one" signal: it starts a `passive` agent that has never run, and
    /// restarts a deactivated one whatever its activation mode - otherwise
    /// Deactivate would be a trap with no way back short of Restart.
    ///
    /// Deliberately not on the layout-restore path, which sets the field
    /// directly: restoring a window must not start what the user has not
    /// asked for.
    pub(super) fn select_agent(&mut self, id: Uuid) {
        self.selected_agent = Some(id);
        self.store.lock().set_activated(id, true);
    }

    /// Stops `id`'s session without removing the agent, per
    /// `agent-lifecycle`'s "Deactivating an agent" requirement: the agent
    /// keeps its name, folder, ordering, persona and activation mode, and
    /// its place in every workspace. Clearing `activated` is what stops the
    /// repaint poll from starting it straight back up; selecting the row
    /// sets it again, in either activation mode.
    ///
    /// Companions go first, mirroring the removal cascade - a companion has
    /// no session worth keeping once its owner's is gone.
    pub(super) fn deactivate_agent(&mut self, id: Uuid) {
        let deactivated = self.store.lock().deactivate(id);
        for agent_id in deactivated {
            self.teardown_session(agent_id);
        }
    }

    /// Writes the store's current agents and workspaces back to settings.
    pub(super) fn persist_agents(&mut self) {
        // Scoped so the store guard is released before the settings file is
        // written: `persist` is blocking I/O, and nothing else should wait on
        // the roster while it runs.
        {
            let store = self.store.lock();
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        if let Err(error) = self.settings.persist() {
            // Not surfaced in the window: the roster is written after every
            // change, so the next one retries, and a dialog per keystroke
            // would be worse than the loss it warns about. Logged because a
            // failure here is what makes a relaunch come back empty.
            eprintln!("failed to persist the agent roster: {error}");
        }
    }

    /// Every agent id in this window's workspace, snapshotted.
    pub(super) fn workspace_agent_ids(&self) -> Vec<Uuid> {
        workspace_agent_ids(&self.store.lock(), self.workspace_id)
    }

    /// Restarts every agent in the workspace, per `agent-list-ui`'s "Restart
    /// All" requirement - the row menu's Restart Agent applied once per
    /// agent, with a single persist at the end.
    pub(super) fn restart_all_agents(&mut self) {
        let ids = self.workspace_agent_ids();
        {
            // One lock for the whole roster rather than one per agent: the
            // restarts are independent, and reacquiring between them lets
            // another window see the workspace half restarted.
            let mut store = self.store.lock();
            for id in &ids {
                if let Err(error) = store.restart(*id) {
                    eprintln!("failed to restart agent {id}: {error}");
                }
            }
        }
        for id in ids {
            self.remove_session(id);
            self.panel_states.remove(&id);
        }
        // `restart` clears the persisted session ids; write them out so a
        // relaunch doesn't resume the sessions just dropped.
        self.persist_agents();
    }

    /// Removes every agent in the workspace, and every companion those
    /// agents own, per `agent-list-ui`'s "Close All" requirement.
    pub(super) fn close_all_agents(&mut self) {
        for id in self.workspace_agent_ids() {
            // A companion is removed with its owner, so by the time the
            // loop reaches one it may already be gone - removing an id the
            // store no longer holds removes nothing.
            self.remove_agent_unpersisted(id);
        }
        self.persist_agents();
    }

    /// Deactivates every running agent in the workspace. An agent that is
    /// not running is left alone rather than treated as a failure, per
    /// `agent-list-ui`'s "Deactivate All" requirement.
    pub(super) fn deactivate_all_agents(&mut self) {
        for id in self.workspace_agent_ids() {
            let running = self.store
                              .lock()
                              .agent(id)
                              .map(|agent| agent.activated)
                              .unwrap_or(false);
            if running {
                self.deactivate_agent(id);
            }
        }
    }

    /// Delivers `text` to every agent in the workspace the way the user
    /// typing it into that agent's own composer would: a prompt to an agent
    /// driven by an ACP panel session, injected text to one running in a
    /// terminal.
    ///
    /// An agent with neither is skipped quietly - one unreachable agent is
    /// not a reason to withhold the message from the rest.
    pub(super) fn broadcast_to_agents(&mut self, text: &str) {
        for id in self.workspace_agent_ids() {
            if self.deliver_panel_prompt(id, text.to_string(), PromptOrigin::User) {
                continue;
            }
            if let Some(session) = self.sessions.get(&id) {
                let mut session = session.lock();
                if let Err(error) = session.send_text(text) {
                    eprintln!("failed to broadcast to agent {id}: {error}");
                }
            }
        }
    }
}
