//! `plan-tasks` and `task-status`: committing a plan and reading it back.

use knot_agents::AgentStore;
use knot_core::Capabilities;
use knot_mcp::ToolCallResult;
use knot_tasks::{Assignee, Task, TaskGraph, TaskId, TaskSpec};
use uuid::Uuid;

use crate::args::require_str;
use crate::lookup::{agent_not_found, find_by_name_or_id, find_in_workspace};
use crate::responses::{PlanTasksResponse, TaskInfo, TaskStatusResponse, success};
use crate::tasks::store::GraphStore;

/// Projects a task into the wire shape.
pub fn task_info(task: &Task) -> TaskInfo {
    let (assignee, capabilities) = match &task.assignee {
        Some(Assignee::Agent(id)) => (Some(id.to_string()), Vec::new()),
        Some(Assignee::Capabilities(tags)) => (None, tags.iter().cloned().collect::<Vec<String>>()),
        None => (None, Vec::new()),
    };
    TaskInfo { id: task.id.to_string(),
               goal: task.goal.clone(),
               state: task.state.to_string(),
               assignee,
               capabilities,
               depends_on: task.depends_on.iter().map(TaskId::to_string).collect(),
               dispatched_to: task.dispatched_to.map(|id| id.to_string()) }
}

fn parse_specs(agents: &AgentStore, caller: Uuid, raw: &[serde_json::Value])
               -> Result<Vec<TaskSpec>, ToolCallResult> {
    raw.iter()
       .map(|task| parse_spec(agents, caller, task))
       .collect()
}

fn parse_spec(agents: &AgentStore, caller: Uuid, raw: &serde_json::Value)
              -> Result<TaskSpec, ToolCallResult> {
    let id = require_str(raw, "id")?;
    let goal = require_str(raw, "goal")?;
    let mut spec = TaskSpec::new(id, goal);

    // An explicit assignee is resolved here, because the caller named
    // something concrete and a name that resolves to nothing is a mistake
    // worth reporting while the plan is being written. Capability tags are
    // deliberately *not* resolved until dispatch.
    if let Some(assignee) = raw.get("assignee").and_then(serde_json::Value::as_str) {
        let Some(agent) = find_in_workspace(agents, caller, assignee)
        else {
            return Err(ToolCallResult::error(format!("Task '{id}' is assigned to '{assignee}', which is not an agent you can see.")));
        };
        spec = spec.assigned_to(agent.id);
    }
    else if let Some(tags) = raw.get("capabilities")
                                  .and_then(serde_json::Value::as_array)
    {
        let tags: Capabilities = tags.iter().filter_map(serde_json::Value::as_str).collect();
        if !tags.is_empty() {
            spec = spec.for_capabilities(tags);
        }
    }

    if let Some(dependencies) = raw.get("dependsOn").and_then(serde_json::Value::as_array) {
        spec = spec.after(dependencies.iter()
                                      .filter_map(serde_json::Value::as_str)
                                      .map(TaskId::new));
    }
    Ok(spec)
}

pub fn plan_tasks(agents: &AgentStore, graphs: &mut GraphStore, arguments: &serde_json::Value)
                  -> ToolCallResult {
    graphs.prune(agents);
    let caller_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Some(caller) = find_by_name_or_id(agents, caller_id_str)
    else {
        return agent_not_found(agents, caller_id_str);
    };
    let caller = caller.id;
    let Some(raw) = arguments.get("tasks").and_then(serde_json::Value::as_array)
    else {
        return ToolCallResult::error("Missing required parameter: tasks");
    };

    let specs = match parse_specs(agents, caller, raw) {
        Ok(specs) => specs,
        Err(error) => return error,
    };
    // Commit against the existing plan when there is one, so tasks already
    // under way keep their state and are neither cancelled nor re-sent.
    let committed = match graphs.get(caller) {
        Some(existing) => existing.recommit(specs),
        None => TaskGraph::commit(specs),
    };
    let graph = match committed {
        Ok(graph) => graph,
        // A rejected commit leaves the previous plan in place: a bad plan
        // must not be able to destroy a good one.
        Err(error) => return ToolCallResult::error(error.to_string()),
    };
    let tasks = graph.tasks().iter().map(task_info).collect();
    graphs.set(caller, graph);
    success(&PlanTasksResponse { tasks })
}

pub fn task_status(agents: &AgentStore, graphs: &mut GraphStore, arguments: &serde_json::Value)
                   -> ToolCallResult {
    graphs.prune(agents);
    let caller_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Some(caller) = find_by_name_or_id(agents, caller_id_str)
    else {
        return agent_not_found(agents, caller_id_str);
    };
    // Asking before planning is not an error: an orchestrator recovering
    // its bearings should get an empty plan, not a failure.
    let tasks = graphs.get(caller.id)
                      .map(|graph| graph.tasks().iter().map(task_info).collect())
                      .unwrap_or_default();
    success(&TaskStatusResponse { tasks })
}
