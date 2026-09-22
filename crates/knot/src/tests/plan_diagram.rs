//! The plan pane end to end: a plan the task tools emit is parsed back and
//! laid out, and its states travel with it.
//!
//! The renderer itself needs a window; the two halves that can be wrong
//! without one - recovering the graph and arranging it - are what these
//! cover. Pins `openspec/specs/task-graph/spec.md`'s "A committed plan is
//! shown to the user".

use knot_agents::{AgentStore, CreateOptions};
use knot_core::l10n;

use crate::plan_view::{layout, mermaid};

/// The dialect is a contract between two crates, so this drives the real
/// emitter rather than a hand-written string.
fn emitted_plan() -> String {
    use knot_tasks::{TaskGraph, TaskId, TaskSpec};

    let graph = TaskGraph::commit(vec![TaskSpec::new("research", "Research the API"),
                                       TaskSpec::new("scaffold", "Scaffold the module"),
                                       TaskSpec::new("implement", "Implement it")
                                           .after([TaskId::new("research"),
                                                   TaskId::new("scaffold")]),
                                       TaskSpec::new("review", "Review the diff")
                                           .after([TaskId::new("implement")])]).unwrap();
    knot_mcp_tools::plan_mermaid(&graph)
}

/// "The plan appears on commit": four tasks and the edges between them.
#[test]
fn a_committed_plan_parses_back_into_its_tasks_and_edges() {
    let graph = mermaid::parse(&emitted_plan());

    assert_eq!(graph.nodes.len(), 4);
    assert_eq!(graph.nodes[0].label, "Research the API");
    // research -> implement, scaffold -> implement, implement -> review
    assert_eq!(graph.edges.len(), 3);
}

#[test]
fn a_committed_plan_lays_out_as_three_columns() {
    let graph = mermaid::parse(&emitted_plan());

    let laid = layout::layout(&graph);

    let depths: Vec<usize> = laid.cards.iter().map(|card| card.depth).collect();
    assert_eq!(depths, vec![0, 0, 1, 2]);
    assert!(laid.width > 0. && laid.height > 0.);
}

/// "The diagram tracks state": a task reported done comes back as done.
#[test]
fn a_completed_task_reads_as_done_in_the_diagram() {
    use knot_tasks::{Outcome, TaskGraph, TaskId, TaskSpec};
    use uuid::Uuid;

    let mut plan = TaskGraph::commit(vec![TaskSpec::new("a", "Research")]).unwrap();
    assert_eq!(mermaid::parse(&knot_mcp_tools::plan_mermaid(&plan)).nodes[0].state
                                                                            .as_deref(),
               Some("ready"));

    plan.mark_dispatched(&TaskId::new("a"), Uuid::new_v4())
        .unwrap();
    plan.complete(&TaskId::new("a"), Outcome::Done).unwrap();

    let graph = mermaid::parse(&knot_mcp_tools::plan_mermaid(&plan));
    assert_eq!(graph.nodes[0].state.as_deref(), Some("done"));
}

/// Dispatching writes the plan into the owner's panel, so the diagram the
/// user sees is the plan as it stands rather than as it was committed.
#[test]
fn the_panel_state_carries_the_plan_after_a_state_change() {
    let mut store = AgentStore::new();
    let lead = store.create("/repo/lead",
                            CreateOptions { name: Some("lead".to_string()),
                                            ..Default::default() });
    store.set_registered(lead, true);
    let worker = store.create("/repo/worker",
                              CreateOptions { name: Some("worker".to_string()),
                                              insert_after: Some(lead),
                                              ..Default::default() });
    store.set_registered(worker, true);

    let mut graphs = knot_mcp_tools::GraphStore::new();
    let plan = serde_json::json!({
        "agentId": lead.to_string(),
        "tasks": [{"id": "a", "goal": "Research", "assignee": "worker"}]
    });
    knot_mcp_tools::plan_tasks(&mut store, &mut graphs, &plan);

    let source = store.agent(lead)
                      .and_then(|agent| agent.mermaid_source.clone())
                      .expect("the plan is shown in the owner's panel");
    let parsed = mermaid::parse(&source);
    assert_eq!(parsed.nodes[0].label, "Research");
    assert_eq!(parsed.nodes[0].state.as_deref(), Some("ready"));
}

#[test]
fn the_pane_copy_resolves() {
    for key in ["plan.title", "plan.empty"] {
        assert_ne!(l10n::t(key), key, "{key} is missing from the catalog");
    }
}
