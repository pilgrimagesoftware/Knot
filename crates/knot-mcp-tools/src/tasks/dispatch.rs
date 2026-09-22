//! `dispatch-task` and `complete-task`: the dependency gate, delivery
//! through the existing messaging rules, and what an outcome changes
//! downstream.

use knot_agents::{AgentStore, RegistryQuery, RegistryView};
use knot_core::BenchAgent;
use knot_mcp::ToolCallResult;
use knot_messaging::{DeliveryNotifier, MessageStore, send};
use knot_tasks::{Assignee, Outcome, TaskId};

use crate::agents::registry::declared_tools;
use crate::args::require_str;
use crate::lookup::{agent_not_found, find_by_name_or_id, workspace_members};
use crate::responses::{CompleteTaskResponse, DispatchTaskResponse, success};
use crate::tasks::store::GraphStore;

/// Everything `dispatch-task` needs from its caller's side of the app.
///
/// Grouped rather than passed as six arguments, per the project's
/// argument-count convention.
pub struct DispatchContext<'a> {
    pub agents:   &'a AgentStore,
    pub graphs:   &'a mut GraphStore,
    pub messages: &'a mut MessageStore,
    pub notifier: &'a dyn DeliveryNotifier,
    pub bench:    &'a [BenchAgent],
}

pub fn dispatch_task(ctx: DispatchContext<'_>, arguments: &serde_json::Value) -> ToolCallResult {
    ctx.graphs.prune(ctx.agents);
    let caller_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let task_id_str = match require_str(arguments, "taskId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let Some(caller) = find_by_name_or_id(ctx.agents, caller_id_str)
    else {
        return agent_not_found(ctx.agents, caller_id_str);
    };
    let caller = caller.clone();
    let task_id = TaskId::new(task_id_str);

    let Some(graph) = ctx.graphs.get(caller.id)
    else {
        return ToolCallResult::error(
                                     "You have no committed plan. Work that spans more than one agent, or more than one \
             task, must go through a plan: call plan-tasks first. A single piece of work for a \
             single agent can go by send-message instead.",
        );
    };
    // Reads only. Nothing moves until delivery actually succeeds, so a
    // rejected delivery leaves the task ready to dispatch again.
    let dispatchable = match graph.gate(&task_id) {
        Ok(dispatchable) => dispatchable,
        Err(error) => return ToolCallResult::error(error.to_string()),
    };

    let recipient = match &dispatchable.assignee {
        Some(Assignee::Agent(id)) => *id,
        Some(Assignee::Capabilities(tags)) => {
            let query = RegistryQuery { capabilities:      tags.clone(),
                                        include_templates: false, };
            let view = RegistryView::new(ctx.agents, ctx.bench, &declared_tools);
            match view.candidates(caller.id, &query).first() {
                Some(candidate) => candidate.id,
                None => {
                    return ToolCallResult::error(format!(
                        "No agent you can see carries every capability task '{task_id}' asks \
                         for. Use describe-agents to see what is available, or assign the task \
                         to an agent directly."
                    ));
                }
            }
        }
        None => {
            return ToolCallResult::error(format!(
                "Task '{task_id}' has no assignee. Re-plan it with an agent or with the \
                 capabilities an agent must carry."
            ));
        }
    };

    // Delivery follows the existing messaging rules unchanged: the sender
    // must be registered, the recipient must share its workspace, shell
    // agents cannot receive, and companion routing applies.
    let members = workspace_members(ctx.agents, caller.id);
    if let Err(error) = send(ctx.messages,
                             ctx.notifier,
                             &caller,
                             &members,
                             recipient,
                             &dispatchable.goal)
    {
        return ToolCallResult::error(error.to_string());
    }

    let graph = ctx.graphs
                   .get_mut(caller.id)
                   .expect("the plan was present a moment ago");
    if let Err(error) = graph.mark_dispatched(&task_id, recipient) {
        return ToolCallResult::error(error.to_string());
    }
    success(&DispatchTaskResponse { success:      true,
                                    task_id:      task_id.to_string(),
                                    recipient_id: recipient.to_string(),
                                    message:      "Task dispatched.".to_string(), })
}

pub fn complete_task(agents: &AgentStore, graphs: &mut GraphStore, arguments: &serde_json::Value)
                     -> ToolCallResult {
    graphs.prune(agents);
    let caller_id_str = match require_str(arguments, "agentId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let task_id_str = match require_str(arguments, "taskId") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let outcome_str = match require_str(arguments, "outcome") {
        Ok(value) => value,
        Err(error) => return error,
    };
    let outcome = match outcome_str {
        "done" => Outcome::Done,
        "failed" => Outcome::Failed,
        other => {
            return ToolCallResult::error(format!("Unknown outcome '{other}'. A task is either 'done' or 'failed'."));
        }
    };
    let Some(caller) = find_by_name_or_id(agents, caller_id_str)
    else {
        return agent_not_found(agents, caller_id_str);
    };
    let Some(graph) = graphs.get_mut(caller.id)
    else {
        return ToolCallResult::error("You have no committed plan.");
    };

    let task_id = TaskId::new(task_id_str);
    match graph.complete(&task_id, outcome) {
        Ok(completion) => {
            success(&CompleteTaskResponse { success:     true,
                                            task_id:     task_id.to_string(),
                                            state:       outcome_str.to_string(),
                                            now_ready:   completion.now_ready
                                                                   .iter()
                                                                   .map(TaskId::to_string)
                                                                   .collect(),
                                            now_blocked: completion.now_blocked
                                                                   .iter()
                                                                   .map(TaskId::to_string)
                                                                   .collect(), })
        }
        Err(error) => ToolCallResult::error(error.to_string()),
    }
}
