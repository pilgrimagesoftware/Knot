//! The registry-facing tools: projecting agents into the wire shape, and
//! the `describe-agents` query behind "who can do X".
//!
//! Implements `openspec/specs/agent-registry/spec.md` and the
//! `describe-agents` requirement in `openspec/specs/mcp-tools/spec.md`.

use knot_agents::{AgentStore, RegistryEntry, RegistryQuery, RegistryStatus, RegistryView};
use knot_core::{BenchAgent, Capabilities};
use knot_mcp::ToolCallResult;
use uuid::Uuid;

use crate::args::require_str;
use crate::lookup::{agent_not_found, find_by_name_or_id};
use crate::responses::{AgentInfo, DescribeAgentsResponse, success};

/// The status string a bench template reports, in place of the automatic
/// state a live agent has.
const TEMPLATE_STATUS: &str = "template";

/// What an agent of this type can reach, beyond the type itself.
///
/// ACP adapters do not declare a per-tool surface to Knot - the adapter
/// registry records how to launch one and which protocol features it
/// supports, and nothing enumerates its tools. So this reports what is
/// actually known: whether the type speaks ACP at all, and the session
/// capabilities its adapter declares. Inventing a tool list would be worse
/// than reporting less, because a caller would choose an agent on it.
///
/// When adapters do report their tools, this is the one function to widen.
pub fn declared_tools(agent_type: &str) -> Vec<String> {
    let Some(adapter) = knot_agent_launch::acp_adapter(agent_type)
    else {
        return Vec::new();
    };
    let mut tools = vec!["acp".to_string()];
    if adapter.supports_resume {
        tools.push("resume".to_string());
    }
    if adapter.supports_permission_modes {
        tools.push("permission-modes".to_string());
    }
    tools
}

/// Projects a registry entry into the wire shape.
pub fn agent_info(entry: &RegistryEntry) -> AgentInfo {
    let (status, is_registered) = match &entry.status {
        RegistryStatus::Live { state,
                               is_registered,
                               .. } => (crate::lookup::state_string(*state), *is_registered),
        RegistryStatus::Template => (TEMPLATE_STATUS.to_string(), false),
    };
    AgentInfo { id: entry.id.to_string(),
                name: entry.name.clone(),
                folder: entry.folder.clone(),
                status,
                is_registered,
                description: entry.description.clone(),
                capabilities: entry.capabilities.iter().cloned().collect(),
                tools: entry.tools.clone(),
                cost_tier: entry.cost_tier.to_string() }
}

/// Every live agent `caller_id` may see, in workspace order.
///
/// Deliberately unranked, unlike `describe-agents`: this is the crew list,
/// and its order is the one the user arranged their agents in.
pub fn visible_agent_infos(store: &AgentStore, caller_id: Uuid) -> Vec<AgentInfo> {
    let view = RegistryView::new(store, &[], &declared_tools);
    knot_agents::visible_to(store, caller_id).into_iter()
                                             .map(|agent| view.entry_for(agent))
                                             .map(|entry| agent_info(&entry))
                                             .collect()
}

pub fn describe_agents(store: &AgentStore, bench: &[BenchAgent], arguments: &serde_json::Value)
                       -> ToolCallResult {
    let caller_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Some(caller) = find_by_name_or_id(store, caller_id_str)
    else {
        return agent_not_found(store, caller_id_str);
    };
    let capabilities: Capabilities =
        arguments.get("capabilities")
                 .and_then(serde_json::Value::as_array)
                 .map(|tags| tags.iter().filter_map(serde_json::Value::as_str).collect())
                 .unwrap_or_default();
    let include_templates = arguments.get("includeTemplates")
                                     .and_then(serde_json::Value::as_bool)
                                     .unwrap_or(true);
    let query = RegistryQuery { capabilities,
                                include_templates };
    let candidates = RegistryView::new(store, bench, &declared_tools).candidates(caller.id, &query)
                                                                     .iter()
                                                                     .map(agent_info)
                                                                     .collect();
    // An empty list is an answer an orchestrator can act on - "there is no
    // reviewer" - not a failure, so this is never an error result.
    success(&DescribeAgentsResponse { candidates })
}

#[cfg(test)]
mod tests;
