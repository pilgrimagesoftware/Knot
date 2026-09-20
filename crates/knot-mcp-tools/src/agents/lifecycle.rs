use knot_agents::AgentStore;
use knot_mcp::ToolCallResult;

use crate::args::require_str;
use crate::lookup::{agent_not_found, find_by_name_or_id};
use crate::responses::{CloseAgentResponse, success};

pub fn close_agent(store: &mut AgentStore, arguments: &serde_json::Value) -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let target_str = match require_str(arguments, "target") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Ok(caller_id) = uuid::Uuid::parse_str(agent_id_str) else {
        return agent_not_found(store, agent_id_str);
    };
    if store.agent(caller_id).is_none() {
        return agent_not_found(store, agent_id_str);
    }
    let Some(target) = crate::lookup::find_in_workspace(store, caller_id, target_str) else {
        return success(&CloseAgentResponse {
            success: false,
            message: format!("Target agent not found: {target_str}"),
        });
    };
    if target.created_by != Some(caller_id) {
        return success(&CloseAgentResponse {
            success: false,
            message: "Permission denied: you can only close agents that you created".to_string(),
        });
    }
    let name = target.name.clone();
    store.remove(target.id);
    success(&CloseAgentResponse {
        success: true,
        message: format!("Agent '{name}' closed successfully"),
    })
}

pub fn set_status(store: &mut AgentStore, arguments: &serde_json::Value) -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let status = match require_str(arguments, "status") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Some(agent) = find_by_name_or_id(store, agent_id_str) else {
        return agent_not_found(store, agent_id_str);
    };
    store.set_status_text(agent.id, status.to_string());
    ToolCallResult::ok("Status updated")
}
