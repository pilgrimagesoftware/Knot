//! Panel-mode (ACP) session lifecycle and the prompts Knot sends of its
//! own accord: the inbox nudge and the MCP registration prompt.
//!
//! Distinct from `sessions.rs`, which owns the PTY side. A panel session
//! outlives a view-mode toggle, so nothing here is driven by which view is
//! on screen.

use super::super::*;

impl WorkspaceWindow {
    /// Starts a Panel-mode ACP connection for `id` if one isn't already
    /// running, per `agent-launch-command`'s "ACP launch path" requirement.
    /// Falls back silently (no session, no error) if the agent type has no
    /// registered adapter - the caller renders the terminal in that case,
    /// and likewise if the agent is not activated (see `ensure_session`).
    pub(in crate::workspace_window) fn ensure_panel_session(&mut self, id: Uuid) {
        if self.panel_sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock().unwrap();
            store.agent(id).cloned()
        };
        let Some(agent) = agent
        else {
            return;
        };
        if !agent.activated {
            return;
        }
        let request = knot_agent_launch::LaunchRequest { agent_type: &agent.agent_type,
                                                         ..Default::default() };
        let knot_agent_launch::LaunchPlan::Adapter(adapter_config) =
            knot_agent_launch::plan_launch(&request)
        else {
            // No registered ACP adapter for this agent type - only shell
            // agents (which never reach `ensure_panel_session`) are meant
            // to fall through to the Terminal path.
            return;
        };
        let mcp_url = self.settings
                          .mcp_server_enabled
                          .then(|| knot_agent_launch::mcp_url(&self.settings));

        let (connecting, progress) = panel_session::PanelSessionSlot::connecting();
        let slot = Arc::new(Mutex::new(connecting));
        self.panel_sessions.insert(id, Arc::clone(&slot));
        let cwd = agent.folder.clone();
        let prior_session_id = agent.acp_session_id.clone();
        let registration_prompt =
            knot_agent_launch::acp_registration_prompt(agent.id,
                                                       prior_session_id.is_some(),
                                                       self.settings.persona(id));
        let store = Arc::clone(&self.store);
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        let request =
                            panel_session::ConnectRequest { config: &adapter_config,
                                                            cwd: &cwd,
                                                            prior_session_id:
                                                                prior_session_id.as_deref(),
                                                            mcp_url: mcp_url.as_deref(),
                                                            registration_prompt };
                        panel_session::connect_into(&slot, request, &progress, |session_id| {
                            if let Ok(mut store) = store.lock() {
                                store.set_acp_session_id(id, session_id.to_string());
                            }
                        }).await;
                    });
    }

    /// Delivers the "check your inbox" prompt to any agent in this
    /// workspace with an unread message it has not been told about, per
    /// `mcp-messaging`'s idle-time delivery nudge.
    ///
    /// Driven from the repaint poll rather than from a send-time event,
    /// because the requirement also covers a message that arrived while its
    /// recipient was working: by the time that agent goes idle the event is
    /// long gone, but the unread message is still in the store to be found.
    pub(in crate::workspace_window) fn deliver_inbox_nudges(&mut self) {
        if !self.settings.mcp_server_enabled {
            return;
        }
        let candidates =
            {
                let (Ok(store), Ok(messages)) = (self.store.lock(), self.messages.lock())
                else {
                    return;
                };
                let Some(workspace) = store.workspaces()
                                           .iter()
                                           .find(|workspace| workspace.id == self.workspace_id)
                else {
                    return;
                };
                workspace
                .agent_ids
                .iter()
                .filter_map(|id| store.agent(*id))
                .filter_map(|agent| {
                    let latest = messages.latest_unread_id(agent.id);
                    let check = app_state::NudgeCheck {
                        agent_type: &agent.agent_type,
                        mcp_enabled: true,
                        latest_message: latest,
                        last_nudged: self.nudged_messages.get(&agent.id).copied(),
                        idle: agent.state == knot_agents::AgentState::Idle,
                        can_receive: self.panel_can_take_a_prompt(agent.id),
                    };
                    app_state::inbox_prompt_message_id(check)
                        .map(|message_id| (agent.id, message_id))
                })
                .collect::<Vec<_>>()
            };
        for (id, message_id) in candidates {
            self.send_inbox_nudge(id);
            self.nudged_messages.insert(id, message_id);
        }
    }

    /// Whether `id`'s panel session could take a prompt this instant: ready,
    /// no permission outstanding, no turn in flight. The same gate the
    /// composer uses - a nudge must not be what discovers a session is busy.
    pub(in crate::workspace_window) fn panel_can_take_a_prompt(&self, id: Uuid) -> bool {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let Ok(guard) = slot.lock()
        else {
            return false;
        };
        match &*guard {
            panel_session::PanelSessionSlot::Ready(handle) => {
                handle.state()
                      .lock()
                      .map(|state| state.pending_permission.is_none() && !state.turn_active)
                      .unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Sends the inbox prompt into `id`'s panel session, recording it in the
    /// conversation the way any other prompt is.
    pub(in crate::workspace_window) fn send_inbox_nudge(&mut self, id: Uuid) {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return;
        };
        let session = {
            let guard = slot.lock().unwrap();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    handle.record_user_message(app_support::CHECK_INBOX_PROMPT.to_string());
                    Some((handle.session(), handle.recorder()))
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session
        else {
            return;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.prompt(app_support::CHECK_INBOX_PROMPT).await {
                            recorder.error(format!("The inbox nudge could not be delivered: \
                                                    {error}"));
                            eprintln!("failed to deliver the inbox nudge: {error}");
                        }
                    });
    }

    /// Sends `id` its MCP registration prompt by hand, for an agent that
    /// failed to register at launch.
    ///
    /// Only Panel-mode agents can be registered this way, which is every
    /// non-shell agent under ACP-only launch - and the menu hides the item
    /// for shell agents. If the session is not `Ready` there is nowhere to
    /// send it; the pane is already showing that connection state, so this
    /// says nothing rather than stacking a second message on top of it.
    pub(in crate::workspace_window) fn send_registration_prompt(&mut self, id: Uuid) {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return;
        };
        let prompt = knot_agent_launch::registration_prompt(id);
        let session = {
            let guard = slot.lock().unwrap();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    handle.record_user_message(prompt.clone());
                    Some((handle.session(), handle.recorder()))
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session
        else {
            return;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.prompt(&prompt).await {
                            recorder.error(format!("The agent could not be registered: {error}"));
                            eprintln!("failed to send the registration prompt: {error}");
                        }
                    });
    }
}
