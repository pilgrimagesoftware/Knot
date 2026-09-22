//! Unit tests for [`super`]. The dialect is a contract with
//! `knot_mcp_tools::tasks::mermaid`, which writes it.

use super::*;

#[test]
fn a_plan_round_trips_from_the_emitted_dialect() {
    let graph = parse("flowchart LR\n    \
                       a[\"Research the API\"]:::done\n    \
                       b[\"Build it\"]:::pending\n    \
                       a --> b\n");

    assert_eq!(graph.nodes,
               vec![PlanNode { id:    "a".to_string(),
                               label: "Research the API".to_string(),
                               state: Some("done".to_string()), },
                    PlanNode { id:    "b".to_string(),
                               label: "Build it".to_string(),
                               state: Some("pending".to_string()), }]);
    assert_eq!(graph.edges, vec![(0, 1)]);
}

#[test]
fn a_node_without_a_state_class_parses() {
    let graph = parse("flowchart LR\n    a[\"just a box\"]\n");

    assert_eq!(graph.nodes[0].state, None);
}

/// An unknown class is kept rather than dropped, so a state added later
/// does not have to be taught to the parser before it can be drawn.
#[test]
fn an_unknown_state_class_is_kept_as_written() {
    let graph = parse("flowchart LR\n    a[\"x\"]:::retrying\n");

    assert_eq!(graph.nodes[0].state.as_deref(), Some("retrying"));
}

/// An edge may be declared before the node it points at, as it is when a
/// plan lists every node first.
#[test]
fn an_edge_declared_before_its_target_still_connects() {
    let graph = parse("flowchart LR\n    a --> b\n    a[\"x\"]\n    b[\"y\"]\n");

    assert_eq!(graph.edges, vec![(0, 1)]);
}

#[test]
fn an_edge_naming_an_unknown_node_is_dropped() {
    let graph = parse("flowchart LR\n    a[\"x\"]\n    a --> ghost\n");

    assert!(graph.edges.is_empty());
}

#[test]
fn a_duplicate_edge_is_recorded_once() {
    let graph = parse("flowchart LR\n    a[\"x\"]\n    b[\"y\"]\n    a --> b\n    a --> b\n");

    assert_eq!(graph.edges, vec![(0, 1)]);
}

#[test]
fn a_self_edge_is_dropped() {
    let graph = parse("flowchart LR\n    a[\"x\"]\n    a --> a\n");

    assert!(graph.edges.is_empty());
}

/// Anything it does not understand is skipped, not fatal: a diagram from
/// some other agent's view-mermaid call should draw as far as it is
/// understood.
#[test]
fn unrecognised_lines_are_ignored_rather_than_fatal() {
    let graph = parse("graph TD\n\
                       %% a comment\n    \
                       subgraph cluster\n    \
                       a[\"x\"]\n    \
                       end\n    \
                       classDef done fill:#0f0\n");

    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.nodes[0].label, "x");
}

#[test]
fn an_empty_or_unparseable_document_is_an_empty_graph() {
    assert!(parse("").is_empty());
    assert!(parse("flowchart LR\n").is_empty());
    assert!(parse("not mermaid at all").is_empty());
}

/// Mermaid shapes other than `["..."]` are not part of the dialect, and a
/// node id is an identifier - neither should be half-read.
#[test]
fn a_shape_outside_the_dialect_is_not_mistaken_for_a_node() {
    assert!(parse("flowchart LR\n    a(rounded)\n").is_empty());
    assert!(parse("flowchart LR\n    a b[\"spaced id\"]\n").is_empty());
}
