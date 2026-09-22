//! Unit tests for [`super`], pinning the task-tool requirements in
//! `openspec/specs/mcp-tools/spec.md` and the gate in
//! `openspec/specs/task-graph/spec.md`.

use knot_agents::{AgentStore, CreateOptions};
use knot_core::CostTier;
use knot_messaging::{MessageStore, NoopNotifier};
use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::tasks::dispatch::DispatchContext;

/// Registered, because messaging rejects an unregistered sender.
fn agent(store: &mut AgentStore, name: &str, tags: &[&str], beside: Option<Uuid>) -> Uuid {
    let id = store.create(format!("/repo/{name}"),
                          CreateOptions { name: Some(name.to_string()),
                                          capabilities: tags.iter().collect(),
                                          cost_tier: CostTier::Medium,
                                          insert_after: beside,
                                          ..Default::default() });
    store.set_registered(id, true);
    id
}

struct Fixture {
    agents:   AgentStore,
    graphs:   GraphStore,
    messages: MessageStore,
    lead:     Uuid,
    worker:   Uuid,
}

fn fixture() -> Fixture {
    let mut agents = AgentStore::new();
    let lead = agent(&mut agents, "lead", &[], None);
    let worker = agent(&mut agents, "worker", &["rust"], Some(lead));
    Fixture { agents,
              graphs: GraphStore::new(),
              messages: MessageStore::new(),
              lead,
              worker }
}

impl Fixture {
    fn plan(&mut self, tasks: serde_json::Value) -> ToolCallResult {
        plan_tasks(&mut self.agents,
                   &mut self.graphs,
                   &json!({"agentId": self.lead.to_string(), "tasks": tasks}))
    }

    fn dispatch(&mut self, task_id: &str) -> ToolCallResult {
        let bench = Vec::new();
        dispatch_task(DispatchContext { agents:   &mut self.agents,
                                        graphs:   &mut self.graphs,
                                        messages: &mut self.messages,
                                        notifier: &NoopNotifier,
                                        bench:    &bench, },
                      &json!({"agentId": self.lead.to_string(), "taskId": task_id}))
    }

    fn complete(&mut self, task_id: &str, outcome: &str) -> ToolCallResult {
        complete_task(&mut self.agents,
                      &mut self.graphs,
                      &json!({"agentId": self.lead.to_string(),
                              "taskId": task_id,
                              "outcome": outcome}))
    }

    fn status(&mut self) -> ToolCallResult {
        task_status(&self.agents,
                    &mut self.graphs,
                    &json!({"agentId": self.lead.to_string()}))
    }
}

use knot_mcp::ToolCallResult;

fn body(result: &ToolCallResult) -> serde_json::Value {
    assert_eq!(result.is_error, None, "{}", result.content[0].text);
    serde_json::from_str(&result.content[0].text).unwrap()
}

fn states(result: &ToolCallResult) -> Vec<(String, String)> {
    body(result)["tasks"].as_array()
                         .unwrap()
                         .iter()
                         .map(|t| {
                             (t["id"].as_str().unwrap().to_string(),
                              t["state"].as_str().unwrap().to_string())
                         })
                         .collect()
}

fn error_text(result: &ToolCallResult) -> String {
    assert_eq!(result.is_error, Some(true));
    result.content[0].text.clone()
}

// ---------------------------------------------------------------------------
// plan-tasks
// ---------------------------------------------------------------------------

/// "A valid plan is committed".
#[test]
fn a_valid_plan_reports_each_task_and_its_starting_state() {
    let mut f = fixture();

    let result = f.plan(json!([{"id": "a", "goal": "research"},
                               {"id": "b", "goal": "build"},
                               {"id": "c", "goal": "review", "dependsOn": ["a", "b"]}]));

    assert_eq!(states(&result),
               vec![("a".to_string(), "ready".to_string()),
                    ("b".to_string(), "ready".to_string()),
                    ("c".to_string(), "pending".to_string())]);
}

/// "A cyclic plan is a tool error".
#[test]
fn a_cyclic_plan_is_a_tool_error_naming_the_cycle() {
    let mut f = fixture();

    let result = f.plan(json!([{"id": "a", "goal": "x", "dependsOn": ["b"]},
                               {"id": "b", "goal": "y", "dependsOn": ["a"]}]));

    let text = error_text(&result);
    assert!(text.contains("cycle"), "{text}");
    assert!(text.contains('a') && text.contains('b'), "{text}");
}

#[test]
fn a_dangling_dependency_is_a_tool_error_naming_it() {
    let mut f = fixture();

    let result = f.plan(json!([{"id": "a", "goal": "x", "dependsOn": ["ghost"]}]));

    assert!(error_text(&result).contains("ghost"));
}

#[test]
fn a_task_may_name_an_agent_directly() {
    let mut f = fixture();
    let worker = f.worker;

    let result = f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));

    assert_eq!(body(&result)["tasks"][0]["assignee"], worker.to_string());
}

/// A name that resolves to nothing is worth reporting while the plan is
/// being written, not at dispatch.
#[test]
fn an_assignee_the_caller_cannot_see_is_refused_at_plan_time() {
    let mut f = fixture();

    let result = f.plan(json!([{"id": "a", "goal": "x", "assignee": "nobody"}]));

    assert!(error_text(&result).contains("nobody"));
}

#[test]
fn capability_tags_are_kept_unresolved_in_the_plan() {
    let mut f = fixture();

    let result = f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["Rust"]}]));

    let task = &body(&result)["tasks"][0];
    assert!(task["assignee"].is_null());
    assert_eq!(task["capabilities"][0], "rust",
               "normalized, still unresolved");
}

#[test]
fn plan_tasks_requires_a_tasks_array() {
    let mut f = fixture();
    let lead = f.lead;

    let result = plan_tasks(&mut f.agents,
                            &mut f.graphs,
                            &json!({"agentId": lead.to_string()}));

    assert!(error_text(&result).contains("tasks"));
}

/// A bad plan must not be able to destroy a good one.
#[test]
fn a_rejected_replan_leaves_the_committed_plan_in_place() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x"}]));

    f.plan(json!([{"id": "p", "goal": "x", "dependsOn": ["q"]},
                  {"id": "q", "goal": "y", "dependsOn": ["p"]}]));

    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

// ---------------------------------------------------------------------------
// dispatch-task
// ---------------------------------------------------------------------------

/// "Ready task is delivered".
#[test]
fn a_ready_task_is_delivered_and_marked_dispatched() {
    let mut f = fixture();
    let worker = f.worker;
    f.plan(json!([{"id": "a", "goal": "build the thing", "assignee": "worker"}]));

    let result = f.dispatch("a");

    assert_eq!(body(&result)["recipientId"], worker.to_string());
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "dispatched".to_string())]);
}

/// "Unmet dependency is reported".
#[test]
fn dispatching_behind_an_unmet_dependency_names_it() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"},
                  {"id": "b", "goal": "y", "assignee": "worker", "dependsOn": ["a"]}]));

    let result = f.dispatch("b");

    let text = error_text(&result);
    assert!(text.contains('a'), "{text}");
    assert_eq!(states(&f.status())[1].1, "pending", "b is untouched");
}

/// "A fan-out without a plan is refused".
#[test]
fn dispatching_with_no_committed_plan_says_to_plan_first() {
    let mut f = fixture();

    let result = f.dispatch("a");

    let text = error_text(&result);
    assert!(text.contains("plan-tasks"), "{text}");
}

/// "A tag assignee resolves at dispatch time".
#[test]
fn a_capability_assignee_resolves_against_the_registry_at_dispatch() {
    let mut f = fixture();
    let worker = f.worker;
    f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["rust"]}]));

    let result = f.dispatch("a");

    assert_eq!(body(&result)["recipientId"], worker.to_string());
    assert_eq!(body(&f.status())["tasks"][0]["dispatchedTo"],
               worker.to_string());
}

/// "No candidate for a tag assignee".
#[test]
fn a_capability_no_visible_agent_carries_is_refused_and_leaves_the_task_ready() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "capabilities": ["haskell"]}]));

    let result = f.dispatch("a");

    assert!(error_text(&result).contains("describe-agents"));
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

/// "A rejected delivery does not consume the task". A shell agent cannot
/// receive messages, so the messaging layer refuses and the task must stay
/// dispatchable.
#[test]
fn a_delivery_the_messaging_rules_reject_leaves_the_task_ready() {
    let mut f = fixture();
    let companion = f.agents.create_shell_companion(f.lead).unwrap();
    f.agents.set_registered(companion, true);
    let plan = json!([{"id": "a", "goal": "x", "assignee": companion.to_string()}]);
    f.plan(plan);

    let result = f.dispatch("a");

    assert_eq!(result.is_error, Some(true));
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

#[test]
fn a_task_with_no_assignee_cannot_be_dispatched() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x"}]));

    let result = f.dispatch("a");

    assert!(error_text(&result).contains("no assignee"));
    assert_eq!(states(&f.status()),
               vec![("a".to_string(), "ready".to_string())]);
}

#[test]
fn a_dispatched_task_cannot_be_dispatched_twice() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));
    f.dispatch("a");

    let result = f.dispatch("a");

    assert!(error_text(&result).contains("dispatched"));
}

// ---------------------------------------------------------------------------
// complete-task
// ---------------------------------------------------------------------------

/// "Completion readies a dependent task".
#[test]
fn completing_the_last_dependency_names_what_became_ready() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"},
                  {"id": "b", "goal": "y", "assignee": "worker", "dependsOn": ["a"]}]));
    f.dispatch("a");

    let result = f.complete("a", "done");

    assert_eq!(body(&result)["nowReady"][0], "b");
    assert!(body(&result)["nowBlocked"].as_array().unwrap().is_empty());
}

/// "Failure blocks dependents", including transitively.
#[test]
fn a_failure_names_every_task_it_blocked() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"},
                  {"id": "b", "goal": "y", "assignee": "worker", "dependsOn": ["a"]},
                  {"id": "c", "goal": "z", "assignee": "worker", "dependsOn": ["b"]}]));
    f.dispatch("a");

    let result = f.complete("a", "failed");

    let blocked = body(&result)["nowBlocked"].as_array().unwrap().clone();
    assert_eq!(blocked, vec![json!("b"), json!("c")]);
}

#[test]
fn an_unknown_outcome_is_refused() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));
    f.dispatch("a");

    let result = f.complete("a", "retrying");

    assert!(error_text(&result).contains("retrying"));
}

#[test]
fn an_undispatched_task_has_no_outcome_to_report() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));

    let result = f.complete("a", "done");

    assert!(error_text(&result).contains("ready"));
}

// ---------------------------------------------------------------------------
// task-status
// ---------------------------------------------------------------------------

/// "Status before planning is empty, not an error".
#[test]
fn asking_for_status_before_planning_is_an_empty_plan() {
    let mut f = fixture();

    let result = f.status();

    assert_eq!(result.is_error, None);
    assert!(body(&result)["tasks"].as_array().unwrap().is_empty());
}

#[test]
fn status_reports_dependencies_and_assignees() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"},
                  {"id": "b", "goal": "y", "dependsOn": ["a"], "capabilities": ["rust"]}]));

    let tasks = body(&f.status())["tasks"].clone();

    assert_eq!(tasks[0]["goal"], "x");
    assert_eq!(tasks[1]["dependsOn"][0], "a");
    assert_eq!(tasks[1]["capabilities"][0], "rust");
}

/// A plan belongs to its owner: removing the agent takes the plan with it,
/// and a re-used id cannot inherit one.
#[test]
fn removing_the_owner_discards_its_plan() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x", "assignee": "worker"}]));
    assert_eq!(f.graphs.len(), 1);

    f.agents.remove(f.lead);
    f.graphs.prune(&f.agents);

    assert_eq!(f.graphs.len(), 0);
}

/// One plan per owner: two orchestrators do not share a plan.
#[test]
fn each_agent_has_its_own_plan() {
    let mut f = fixture();
    f.plan(json!([{"id": "a", "goal": "x"}]));

    let other = task_status(&f.agents,
                            &mut f.graphs,
                            &json!({"agentId": f.worker.to_string()}));

    assert!(body(&other)["tasks"].as_array().unwrap().is_empty());
}
