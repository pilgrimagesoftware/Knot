//! Panel-mode (ACP) session lifecycle and the prompts Knot sends of its
//! own accord: the inbox nudge and the MCP registration prompt.
//!
//! Distinct from `sessions.rs`, which owns the PTY side. A panel session
//! outlives a view-mode toggle, so nothing here is driven by which view is
//! on screen.

use std::sync::Arc;

use gpui_kit::App;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_state;
use crate::app_support;
use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

impl WorkspaceWindow {
    /// Starts a Panel-mode ACP connection for `id` if one isn't already
    /// running, per `agent-launch-command`'s "ACP launch path" requirement.
    /// Falls back silently (no session, no error) if the agent type has no
    /// registered adapter - the caller renders the terminal in that case,
    /// and likewise if the agent is not activated (see `ensure_session`).
    pub(in crate::workspace_window) fn ensure_panel_session(&mut self, id: Uuid, cx: &App) {
        if self.panel_sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock();
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
        let mcp_url = crate::settings_global::read(cx)
                          .mcp_server_enabled
                          .then(|| knot_agent_launch::mcp_url(&crate::settings_global::read(cx)));

        let (connecting, progress) = panel_session::PanelSessionSlot::connecting();
        let slot = Arc::new(Mutex::new(connecting));
        self.panel_sessions.insert(id, Arc::clone(&slot));
        let cwd = agent.folder.clone();
        let prior_session_id = agent.acp_session_id.clone();
        let registration_prompt =
            knot_agent_launch::acp_registration_prompt(agent.id,
                                                       prior_session_id.is_some(),
                                                       crate::settings_global::read(cx).persona(id));
        let session_config = agent.session_config.clone();
        // Built here because this is the only place that knows both the
        // agent's id and its type; `None` for a type with no recognizer,
        // which costs the session nothing.
        let subagents = crate::subagent_feed::SubagentSink::new(id,
                                                                &agent.agent_type,
                                                                Arc::clone(&self.subagents));
        let store = Arc::clone(&self.store);
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        let request =
                            panel_session::ConnectRequest { config: &adapter_config,
                                                            cwd: &cwd,
                                                            prior_session_id:
                                                                prior_session_id.as_deref(),
                                                            mcp_url: mcp_url.as_deref(),
                                                            registration_prompt,
                                                            session_config,
                                                            subagents };
                        panel_session::connect_into(&slot, request, &progress, |session_id| {
                            let mut store = store.lock();
                            store.set_acp_session_id(id, session_id.to_string());
                        }).await;
                    });
    }

    /// Records one Panel session-setup selection for `id` and writes the
    /// roster out, per `session-setup-persistence`'s "Session setup
    /// persists per agent" requirement.
    ///
    /// Persisting is separate from applying: the live session is updated by
    /// the caller through `set_config_option`, and a session that isn't
    /// running yet picks this up when `ensure_panel_session` replays it.
    pub(in crate::workspace_window) fn remember_session_config(&mut self, id: Uuid,
                                                               config_id: String, value: String,
                                                               cx: &App) {
        let recorded = self.store
                           .lock()
                           .set_session_config_option(id, config_id, value)
                           .is_ok();
        if recorded {
            self.persist_agents(cx);
        }
    }

    /// Delivers the "check your inbox" prompt to any agent in this
    /// workspace with an unread message it has not been told about, per
    /// `mcp-messaging`'s idle-time delivery nudge.
    ///
    /// Driven from the repaint poll rather than from a send-time event,
    /// because the requirement also covers a message that arrived while its
    /// recipient was working: by the time that agent goes idle the event is
    /// long gone, but the unread message is still in the store to be found.
    ///
    /// A message is recorded as nudged only when the nudge was actually
    /// taken - sent or queued. An agent with no panel session takes
    /// nothing, and the spec's "delivered when that agent next has a live
    /// session" depends on the message still looking un-nudged next poll.
    pub(in crate::workspace_window) fn deliver_inbox_nudges(&mut self, cx: &App) {
        if !crate::settings_global::read(cx).mcp_server_enabled {
            return;
        }
        let candidates = {
            let (store, messages) = (self.store.lock(), self.messages.lock());
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
                    };
                    app_state::inbox_prompt_message_id(check)
                        .map(|message_id| (agent.id, message_id))
                })
                .collect::<Vec<_>>()
        };
        for (id, message_id) in candidates {
            if self.send_inbox_nudge(id) {
                self.nudged_messages.insert(id, message_id);
            }
        }
    }

    /// Sends the inbox prompt into `id`'s panel session, recording it in the
    /// conversation the way any other prompt is.
    ///
    /// Routed through [`Self::deliver_panel_prompt`] rather than prompting
    /// the session directly, per `mcp-messaging`'s "Inbox nudges preserve
    /// interrupted session work": a turn can start between the poll's
    /// eligibility check and this call, and the direct path would prompt
    /// straight over it. Queueing instead leaves the running turn to
    /// finish, and `deliver_waiting_prompts` delivers the nudge
    /// once it does.
    ///
    /// Returns whether the nudge was taken - sent or queued. `false` means
    /// `id` has no panel session, so nothing was delivered and nothing
    /// should be recorded as nudged.
    pub(in crate::workspace_window) fn send_inbox_nudge(&mut self, id: Uuid) -> bool {
        self.deliver_panel_prompt(id,
                                  app_support::CHECK_INBOX_PROMPT.to_string(),
                                  PromptOrigin::InboxNudge)
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
            let guard = slot.lock();
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
                            recorder.error(knot_core::l10n::t_with("panel.error_registration",
                                                                   &[("error",
                                                                      &error.to_string())]));
                            eprintln!("failed to send the registration prompt: {error}");
                        }
                    });
    }
}
