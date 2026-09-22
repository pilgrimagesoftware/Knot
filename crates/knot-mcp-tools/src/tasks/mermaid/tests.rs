//! Unit tests for [`super`]. The dialect is a contract with
//! `crates/knot/src/plan_view`, which parses it back, so these pin its
//! exact shape.

use knot_tasks::{Outcome, TaskSpec};
use uuid::Uuid;

use super::*;

fn spec(id: &str, goal: &str) -> TaskSpec {
    TaskSpec::new(id, goal)
}

#[test]
fn a_plan_renders_as_nodes_then_edges() {
    let graph = TaskGraph::commit(vec![spec("a", "Research the API"),
                                       spec("b", "Build it").after([TaskId::new("a")])]).unwrap();

    assert_eq!(to_mermaid(&graph),
               "flowchart LR\n    \
                a[\"Research the API\"]:::ready\n    \
                b[\"Build it\"]:::pending\n    \
                a --> b\n");
}

#[test]
fn a_task_state_is_carried_as_its_class() {
    let mut graph = TaskGraph::commit(vec![spec("a", "x")]).unwrap();
    graph.mark_dispatched(&TaskId::new("a"), Uuid::new_v4())
         .unwrap();
    assert!(to_mermaid(&graph).contains(":::dispatched"));

    graph.complete(&TaskId::new("a"), Outcome::Done).unwrap();
    assert!(to_mermaid(&graph).contains(":::done"));
}

#[test]
fn a_fan_in_emits_one_edge_per_dependency() {
    let graph = TaskGraph::commit(vec![spec("a", "x"),
                                       spec("b", "y"),
                                       spec("c", "z").after([TaskId::new("a"),
                                                             TaskId::new("b")])]).unwrap();

    let source = to_mermaid(&graph);
    assert!(source.contains("    a --> c\n"));
    assert!(source.contains("    b --> c\n"));
}

/// A quote would end the label early and a newline would end the line, so
/// both are replaced rather than escaped.
#[test]
fn a_goal_cannot_break_out_of_its_label() {
    let graph = TaskGraph::commit(vec![spec("a", "Say \"hello\"\nthen stop")]).unwrap();

    let source = to_mermaid(&graph);
    assert!(source.contains(r#"a["Say  hello  then stop"]"#), "{source}");
    assert_eq!(source.lines().count(), 2);
}

/// Task ids are whatever the orchestrator typed; mermaid node ids are not.
#[test]
fn an_id_with_punctuation_is_sanitized() {
    let graph = TaskGraph::commit(vec![spec("step one!", "x")]).unwrap();

    assert!(to_mermaid(&graph).contains("step_one_["),
            "{}",
            to_mermaid(&graph));
}

/// Two ids that sanitize alike must not merge into one node.
#[test]
fn ids_that_sanitize_to_the_same_thing_stay_distinct() {
    let graph = TaskGraph::commit(vec![spec("a.b", "first"), spec("a-b", "second")]).unwrap();

    let source = to_mermaid(&graph);
    assert!(source.contains("a_b_0["), "{source}");
    assert!(source.contains("a_b_1["), "{source}");
}

#[test]
fn an_empty_plan_is_just_the_header() {
    let graph = TaskGraph::commit(Vec::new()).unwrap();

    assert_eq!(to_mermaid(&graph), "flowchart LR\n");
}
