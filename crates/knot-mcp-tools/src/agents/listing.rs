use knot_agents::{Agent, AgentStore};
use knot_mcp::ToolCallResult;
use uuid::Uuid;

use crate::args::{optional_str, require_str};
use crate::lookup::{agent_not_found, find_by_name_or_id, state_string, workspace_members};
use crate::responses::{AgentInfo, RegisterAgentResponse, success};

fn agent_info(agent: &Agent) -> AgentInfo {
    AgentInfo { id:            agent.id.to_string(),
                name:          agent.name.clone(),
                folder:        agent.folder.clone(),
                status:        state_string(agent.state),
                is_registered: agent.is_registered, }
}

fn list_agents_response(store: &AgentStore, caller_id: Uuid) -> Vec<AgentInfo> {
    workspace_members(store, caller_id).into_iter()
                                       .filter(|agent| {
                                           !agent.is_companion
                                           || agent.created_by == Some(caller_id)
                                       })
                                       .map(|agent| agent_info(&agent))
                                       .collect()
}

pub fn register_agent(store: &mut AgentStore, arguments: &serde_json::Value) -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Ok(agent_id) = Uuid::parse_str(agent_id_str)
    else {
        return agent_not_found(store, agent_id_str);
    };
    if store.agent(agent_id).is_none() {
        return agent_not_found(store, agent_id_str);
    }

    store.set_registered(agent_id, true);
    if let Some(session_id) = optional_str(arguments, "sessionId") {
        store.set_session_id(agent_id, session_id.to_string());
    }

    success(&RegisterAgentResponse {
        success: true,
        message: "Successfully registered with Knot crew. Note: knot members can change over time as agents join or leave. Use list-agents to get the current list.".to_string(),
        unread_message_count: 0,
        knot_members: list_agents_response(store, agent_id),
    })
}

pub fn list_agents(store: &AgentStore, arguments: &serde_json::Value) -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Some(caller) = find_by_name_or_id(store, agent_id_str)
    else {
        return agent_not_found(store, agent_id_str);
    };
    success(&crate::responses::ListAgentsResponse { agents: list_agents_response(store, caller.id), })
}
