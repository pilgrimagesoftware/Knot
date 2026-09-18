use knot_agents::AgentState;
use knot_mcp::{
    AgentHookHandler, HookRequest, HookStatus, claude_status, codex_turn_complete, extract_metadata,
};

use crate::McpToolCatalog;
use crate::lookup::state_string;

impl AgentHookHandler for McpToolCatalog {
    fn register(&self, request: &HookRequest) -> Result<serde_json::Value, knot_mcp::HookError> {
        if request.agent != "claude" {
            return Err(knot_mcp::HookError::UnknownAgent(request.agent.clone()));
        }
        let id = request.agent_id()?;
        let mut agents = self.agents.lock().unwrap();
        let Some(agent) = agents.agent(id)
        else {
            return Err(knot_mcp::HookError::InvalidPayload("Agent not found".to_string()));
        };
        let is_resuming = agent.resume_session_id.is_some() && !agent.fork_session;
        let is_fork = agent.fork_session;
        let source = request.source.as_deref().unwrap_or("startup");
        agents.set_registered(id, true);
        agents.update_metadata(id, extract_metadata("claude", &request.payload));
        if source == "resume" {
            if !is_fork && let Some(session_id) = request.session_id.clone() {
                agents.set_session_id(id, session_id);
            }
        }
        else if !is_resuming && let Some(session_id) = request.session_id.clone() {
            agents.set_session_id(id, session_id);
        }
        Ok(serde_json::json!({
            "success": true,
            "message": "Registered",
            "unreadMessageCount": 0,
            "knotMembers": agents.agents().iter().map(|agent| serde_json::json!({
                "id": agent.id.to_string(),
                "name": agent.name,
                "folder": agent.folder,
                "status": state_string(agent.state),
                "isRegistered": agent.is_registered,
            })).collect::<Vec<_>>(),
        }))
    }

    fn status(&self, request: &HookRequest) -> Result<serde_json::Value, knot_mcp::HookError> {
        let id = request.agent_id()?;
        if self.agents.lock().unwrap().agent(id).is_none() {
            return Err(knot_mcp::HookError::InvalidPayload("Agent not found".to_string()));
        }
        self.agents
            .lock()
            .unwrap()
            .update_metadata(id, extract_metadata(&request.agent, &request.payload));
        let status = match request.agent.as_str() {
            "claude" => claude_status(
                request
                    .status
                    .as_deref()
                    .ok_or_else(|| knot_mcp::HookError::InvalidStatus("missing".to_string()))?,
            )?,
            "codex" => {
                if let Some(thread_id) = codex_turn_complete(&request.payload)? {
                    self.agents.lock().unwrap().set_session_id(id, thread_id);
                }
                HookStatus::Idle
            }
            agent => return Err(knot_mcp::HookError::UnknownAgent(agent.to_string())),
        };
        let state = match status {
            HookStatus::Working => AgentState::Running,
            HookStatus::Idle => AgentState::Idle,
            HookStatus::AwaitingInput => AgentState::Input,
        };
        let input_message =
            matches!(status, HookStatus::AwaitingInput).then(|| {
                                                           hook_input_message(&request.payload)
                                                       })
                                                       .flatten();
        if self.tracker_for(id, &request.agent) {
            self.trackers
                .lock()
                .unwrap()
                .get(&id)
                .expect("tracker inserted above")
                .apply_hook_status(state, input_message);
        }
        else {
            self.agents.lock().unwrap().set_state(id, state);
        }
        Ok(serde_json::json!({"success": true}))
    }
}

pub(crate) fn hook_input_message(payload: &serde_json::Value) -> Option<String> {
    payload.get("message")
           .and_then(serde_json::Value::as_str)
           .map(str::to_owned)
}
