//! Unit tests for [`super`], pinning the `describe-agents` and widened
//! `list-agents` scenarios in `openspec/specs/mcp-tools/spec.md`.

use knot_agents::{CreateOptions, EditRequest};
use knot_core::CostTier;
use serde_json::json;

use super::*;

fn tagged(store: &mut AgentStore, name: &str, tags: &[&str], cost: CostTier,
          beside: Option<Uuid>)
          -> Uuid {
    store.create(format!("/repo/{name}"),
                 CreateOptions { name: Some(name.to_string()),
                                 capabilities: tags.iter().collect(),
                                 cost_tier: cost,
                                 insert_after: beside,
                                 ..Default::default() })
}

fn bench_entry(name: &str, tags: &[&str]) -> BenchAgent {
    let mut bench = BenchAgent::new(Uuid::new_v4(), name, None, format!("/repo/{name}"));
    bench.capabilities = tags.iter().collect();
    bench
}

fn candidates(result: &ToolCallResult) -> Vec<serde_json::Value> {
    assert_eq!(result.is_error, None, "{:?}", result.content);
    let parsed: serde_json::Value = serde_json::from_str(&result.content[0].text).unwrap();
    parsed["candidates"].as_array().cloned().unwrap_or_default()
}

fn names(result: &ToolCallResult) -> Vec<String> {
    candidates(result).iter()
                      .map(|c| c["name"].as_str().unwrap().to_string())
                      .collect()
}

/// "Query by tag".
#[test]
fn describe_agents_returns_only_candidates_carrying_the_tag_ranked() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store,
           "expensive",
           &["code-review"],
           CostTier::High,
           Some(caller));
    tagged(&mut store,
           "cheap",
           &["code-review"],
           CostTier::Low,
           Some(caller));
    tagged(&mut store,
           "unrelated",
           &["rust"],
           CostTier::Low,
           Some(caller));

    let result = describe_agents(&store,
                                 &[],
                                 &json!({"agentId": caller.to_string(),
                                         "capabilities": ["code-review"]}));

    assert_eq!(names(&result), vec!["cheap", "expensive"]);
}

/// "Templates can be excluded".
#[test]
fn describe_agents_can_leave_out_bench_templates() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let bench = [bench_entry("template", &["rust"])];
    let args = json!({"agentId": caller.to_string(),
                      "capabilities": ["rust"],
                      "includeTemplates": false});

    let result = describe_agents(&store, &bench, &args);

    assert!(names(&result).is_empty());
}

#[test]
fn describe_agents_includes_templates_by_default() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let bench = [bench_entry("template", &["rust"])];

    let result = describe_agents(&store,
                                 &bench,
                                 &json!({"agentId": caller.to_string(),
                                         "capabilities": ["rust"]}));

    assert_eq!(names(&result), vec!["template"]);
    assert_eq!(candidates(&result)[0]["status"], "template");
    assert_eq!(candidates(&result)[0]["isRegistered"], false);
}

/// "No match is not an error". An orchestrator discovering it has no
/// reviewer needs an answer it can act on, not a failure.
#[test]
fn describe_agents_reports_no_match_as_an_empty_list() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &["rust"], CostTier::Medium, None);

    let result = describe_agents(&store,
                                 &[],
                                 &json!({"agentId": caller.to_string(),
                                         "capabilities": ["code-reveiw"]}));

    assert_eq!(result.is_error, None);
    assert!(candidates(&result).is_empty());
}

#[test]
fn describe_agents_with_no_tags_lists_everything_visible() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store,
           "worker",
           &["rust"],
           CostTier::Medium,
           Some(caller));

    let result = describe_agents(&store, &[], &json!({"agentId": caller.to_string()}));

    assert_eq!(names(&result).len(), 2);
}

#[test]
fn describe_agents_normalizes_the_tags_it_is_given() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    tagged(&mut store,
           "worker",
           &["code-review"],
           CostTier::Low,
           Some(caller));

    let result = describe_agents(&store,
                                 &[],
                                 &json!({"agentId": caller.to_string(),
                                         "capabilities": ["  Code-Review  "]}));

    assert_eq!(names(&result), vec!["worker"]);
}

#[test]
fn describe_agents_requires_a_caller_id() {
    let store = AgentStore::new();

    let result = describe_agents(&store, &[], &json!({}));

    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("agentId"));
}

#[test]
fn describe_agents_rejects_an_unknown_caller() {
    let mut store = AgentStore::new();
    tagged(&mut store, "someone", &[], CostTier::Medium, None);

    let result = describe_agents(&store, &[], &json!({"agentId": "bogus"}));

    assert_eq!(result.is_error, Some(true));
}

/// "Registry fields accompany each agent".
#[test]
fn a_listing_carries_the_registry_fields() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);
    let worker = tagged(&mut store,
                        "tester",
                        &["testing"],
                        CostTier::Low,
                        Some(caller));
    // Through the real edit path: `agent_mut` is `pub(super)` on purpose.
    store.edit(worker,
               EditRequest { name: "tester".to_string(),
                             description: "Runs the test suite".to_string(),
                             capabilities: ["testing"].iter().collect(),
                             cost_tier: CostTier::Low,
                             ..Default::default() })
         .expect("edit succeeds");

    let infos = visible_agent_infos(&store, caller);

    let info = infos.iter().find(|i| i.name == "tester").unwrap();
    assert_eq!(info.description, "Runs the test suite");
    assert_eq!(info.capabilities, vec!["testing"]);
    assert_eq!(info.cost_tier, "low");
    // `claude` is the default type, and its adapter speaks ACP.
    assert_eq!(info.tools,
               vec!["claude", "acp", "resume", "permission-modes"]);
}

/// "An undescribed agent is still listed". Lacking a description must not
/// hide an agent from the crew list.
#[test]
fn an_undescribed_agent_still_appears_with_defaults() {
    let mut store = AgentStore::new();
    let caller = tagged(&mut store, "caller", &[], CostTier::Medium, None);

    let infos = visible_agent_infos(&store, caller);

    let info = &infos[0];
    assert_eq!(info.description, "");
    assert!(info.capabilities.is_empty());
    assert_eq!(info.cost_tier, "medium");
}

/// A type with no ACP adapter reports only itself: nothing is invented for
/// it, because a caller would choose an agent on what this says.
#[test]
fn a_type_with_no_adapter_reports_only_its_type() {
    assert_eq!(declared_tools("shell"), Vec::<String>::new());
    assert!(declared_tools("claude").contains(&"acp".to_string()));
}
