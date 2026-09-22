use knot_agents::{AgentStore, CreateOptions};
use knot_core::BenchAgent;
use knot_mcp::ToolCallResult;
use uuid::Uuid;

use crate::args::{optional_bool, optional_str, require_str};
use crate::lookup::agent_not_found;
use crate::responses::{CreateAgentResponse, success};

struct ResolvedCreateFields {
    name:          Option<String>,
    icon:          Option<String>,
    agent_type:    Option<String>,
    repo_path:     Option<String>,
    shell_command: Option<String>,
    persona_id:    Option<Uuid>,
}

fn resolve_fields(arguments: &serde_json::Value, bench: Option<&BenchAgent>)
                  -> ResolvedCreateFields {
    ResolvedCreateFields { name:          optional_str(arguments, "name").map(str::to_string)
                                                                         .or_else(|| {
                                                                             bench.map(|value| {
                                                                                      value.name
                                                                                           .clone()
                                                                                  })
                                                                         }),
                           icon:          optional_str(arguments, "icon").map(str::to_string)
                                                                         .or_else(|| {
                                                                             bench.map(|value| {
                                                                                      value.avatar
                                                                                           .clone()
                                                                                  })
                                                                         }),
                           agent_type:
                               optional_str(arguments, "agentType").map(str::to_string)
                                                                   .or_else(|| {
                                                                       bench.map(|value| {
                                                                                value.agent_type
                                                                                     .clone()
                                                                            })
                                                                   }),
                           repo_path:     optional_str(arguments, "repoPath").map(str::to_string)
                                                                             .or_else(|| {
                                                                                 bench.map(|value| {
                                                                                     value.folder
                                                                                          .clone()
                                                                                 })
                                                                             }),
                           shell_command:
                               optional_str(arguments, "command").map(str::to_string)
                                                                 .or_else(|| {
                                                                     bench.and_then(|value| {
                                                                              value.shell_command
                                                                                   .clone()
                                                                          })
                                                                 }),
                           persona_id:
                               optional_str(arguments, "personaId").and_then(|value| {
                                                                       Uuid::parse_str(value).ok()
                                                                   })
                                                                   .or_else(|| {
                                                                       bench.and_then(|value| {
                                                                                value.persona_id
                                                                            })
                                                                   }), }
}

pub fn create_agent(store: &mut AgentStore, arguments: &serde_json::Value,
                    bench_agents: &[BenchAgent])
                    -> ToolCallResult {
    let agent_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Ok(created_by) = Uuid::parse_str(agent_id_str)
    else {
        return agent_not_found(store, agent_id_str);
    };
    let bench = match optional_str(arguments, "benchAgentId") {
        Some(id_str) => {
            let Ok(bench_id) = Uuid::parse_str(id_str)
            else {
                return ToolCallResult::error(format!("Bench agent not found: {id_str}"));
            };
            match bench_agents.iter().find(|bench| bench.id == bench_id) {
                Some(bench) => Some(bench),
                None => return ToolCallResult::error(format!("Bench agent not found: {id_str}")),
            }
        }
        None => None,
    };
    let fields = resolve_fields(arguments, bench);

    let mut missing = Vec::new();
    if fields.name.is_none() {
        missing.push("name");
    }
    if fields.agent_type.is_none() {
        missing.push("agentType");
    }
    if fields.repo_path.is_none() {
        missing.push("repoPath");
    }
    if !missing.is_empty() {
        return ToolCallResult::error(format!("Missing required parameters: {}. Provide these or use benchAgentId to deploy from a template.",
                                             missing.join(", ")));
    }

    let create_worktree = optional_bool(arguments, "createWorktree").unwrap_or(false);
    let branch_name = optional_str(arguments, "branchName");
    if create_worktree && branch_name.is_none_or(str::is_empty) {
        return ToolCallResult::error("Missing required parameter: branchName");
    }
    let companion = optional_bool(arguments, "companion").unwrap_or(false);
    if companion && fields.agent_type.as_deref() != Some("shell") {
        return ToolCallResult::error("Companion agents must use agentType=shell");
    }

    // Destructured rather than unwrapped: the `missing` check above already
    // proves `repo_path` is present, and `branch_name` is present whenever
    // `create_worktree` is - but nothing tied either to its check, so
    // reordering this function reintroduced a panic. The compiler holds the
    // pairing now, and these arms are the checks' own messages.
    let Some(repo_path) = fields.repo_path
    else {
        return ToolCallResult::error("Missing required parameter: repoPath");
    };
    let folder = match (create_worktree, branch_name) {
        (true, Some(branch)) => match crate::repos::create_worktree_path(&repo_path, branch) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(error) => {
                return ToolCallResult::error(format!("Failed to create worktree: {error}"));
            }
        },
        (true, None) => return ToolCallResult::error("Missing required parameter: branchName"),
        (false, _) => repo_path,
    };
    let id = store.create(folder,
                          CreateOptions { name: fields.name,
                                          avatar: fields.icon,
                                          agent_type: fields.agent_type,
                                          shell_command: fields.shell_command,
                                          persona_id: fields.persona_id,
                                          created_by: Some(created_by),
                                          is_companion: companion,
                                          insert_after: None,
                                          workspace_id: None,
                                          // Active, not the dialog's
                                          // `Passive` default: an agent
                                          // created over MCP was asked for
                                          // by another agent, and there is
                                          // no user to select its row.
                                          activation_mode: knot_core::ActivationMode::Active,
                                          // Registry metadata is not yet a
                                          // `create-agent` argument; an
                                          // agent created over MCP starts
                                          // undescribed and untagged.
                                          ..Default::default() });
    success(&CreateAgentResponse { success:  true,
                                   agent_id: Some(id.to_string()),
                                   message:  "Agent created successfully".to_string(), })
}
