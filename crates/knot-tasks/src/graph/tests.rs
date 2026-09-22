//! Unit tests for [`super`], pinning the scenarios in
//! `openspec/specs/task-graph/spec.md`.

use super::*;

fn spec(id: &str) -> TaskSpec {
    TaskSpec::new(id, format!("do {id}"))
}

fn after(id: &str, dependencies: &[&str]) -> TaskSpec {
    spec(id).after(dependencies.iter().map(|d| TaskId::new(*d)))
}

fn id(name: &str) -> TaskId {
    TaskId::new(name)
}

fn state_of(graph: &TaskGraph, name: &str) -> TaskState {
    graph.task(&id(name)).expect("task is present").state
}

/// Drives a task all the way through, the way a caller would.
fn finish(graph: &mut TaskGraph, name: &str, outcome: Outcome) -> Completion {
    graph.mark_dispatched(&id(name), Uuid::new_v4())
         .expect("dispatchable");
    graph.complete(&id(name), outcome).expect("dispatched")
}

// ---------------------------------------------------------------------------
// Commit-time validation
// ---------------------------------------------------------------------------

/// "A cycle is refused".
#[test]
fn a_cycle_is_refused_and_named() {
    let error = TaskGraph::commit(vec![after("a", &["b"]), after("b", &["a"])]).unwrap_err();

    match error {
        TaskError::Cycle(cycle) => {
            assert!(cycle.contains(&id("a")), "{cycle:?}");
            assert!(cycle.contains(&id("b")), "{cycle:?}");
        }
        other => panic!("expected a cycle, got {other:?}"),
    }
}

#[test]
fn a_longer_cycle_is_refused() {
    let error =
        TaskGraph::commit(vec![after("a", &["c"]), after("b", &["a"]), after("c", &["b"])]).unwrap_err();

    assert!(matches!(error, TaskError::Cycle(_)), "{error:?}");
}

/// "A dangling dependency is refused".
#[test]
fn a_dependency_on_a_task_not_in_the_plan_is_refused_and_named() {
    let error = TaskGraph::commit(vec![after("a", &["ghost"])]).unwrap_err();

    assert_eq!(error,
               TaskError::UnknownDependency { task:    id("a"),
                                              missing: id("ghost"), });
}

/// "A self-dependency is refused".
#[test]
fn a_task_depending_on_itself_is_refused() {
    let error = TaskGraph::commit(vec![after("a", &["a"])]).unwrap_err();

    assert_eq!(error, TaskError::SelfDependency(id("a")));
}

#[test]
fn a_duplicate_id_is_refused() {
    let error = TaskGraph::commit(vec![spec("a"), spec("a")]).unwrap_err();

    assert_eq!(error, TaskError::DuplicateId(id("a")));
}

#[test]
fn a_plan_past_the_task_limit_is_refused() {
    let specs: Vec<TaskSpec> = (0..=crate::consts::MAX_TASKS).map(|n| spec(&n.to_string()))
                                                             .collect();

    let error = TaskGraph::commit(specs).unwrap_err();

    assert!(matches!(error, TaskError::TooManyTasks { .. }), "{error:?}");
}

/// A rejected commit must not be able to destroy a plan that already works.
#[test]
fn a_rejected_commit_leaves_the_previous_plan_untouched() {
    let graph = TaskGraph::commit(vec![spec("a"), after("b", &["a"])]).unwrap();

    let error = graph.recommit(vec![after("x", &["y"]), after("y", &["x"])])
                     .unwrap_err();

    assert!(matches!(error, TaskError::Cycle(_)));
    assert_eq!(state_of(&graph, "a"), TaskState::Ready);
    assert_eq!(state_of(&graph, "b"), TaskState::Pending);
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

#[test]
fn committing_readies_the_independent_tasks_and_pends_the_rest() {
    let graph = TaskGraph::commit(vec![spec("a"), spec("b"), after("c", &["a", "b"])]).unwrap();

    assert_eq!(state_of(&graph, "a"), TaskState::Ready);
    assert_eq!(state_of(&graph, "b"), TaskState::Ready);
    assert_eq!(state_of(&graph, "c"), TaskState::Pending);
}

/// "Completing the last dependency readies a task".
#[test]
fn a_task_becomes_ready_when_its_last_dependency_is_done() {
    let mut graph = TaskGraph::commit(vec![spec("a"), spec("b"), after("c", &["a", "b"])]).unwrap();

    let first = finish(&mut graph, "a", Outcome::Done);
    assert!(first.now_ready.is_empty(), "c still waits on b");
    assert_eq!(state_of(&graph, "c"), TaskState::Pending);

    let second = finish(&mut graph, "b", Outcome::Done);
    assert_eq!(second.now_ready, vec![id("c")]);
    assert_eq!(state_of(&graph, "c"), TaskState::Ready);
}

/// "A failure blocks everything downstream", directly and transitively.
#[test]
fn a_failure_blocks_dependents_directly_and_transitively() {
    let mut graph =
        TaskGraph::commit(vec![spec("a"), after("b", &["a"]), after("c", &["b"])]).unwrap();

    let completion = finish(&mut graph, "a", Outcome::Failed);

    assert_eq!(completion.now_blocked, vec![id("b"), id("c")]);
    assert_eq!(state_of(&graph, "b"), TaskState::Blocked);
    assert_eq!(state_of(&graph, "c"), TaskState::Blocked);
}

/// Submission order must not decide how far a failure propagates.
#[test]
fn blocking_reaches_dependents_declared_before_the_task_that_failed() {
    let mut graph =
        TaskGraph::commit(vec![after("c", &["b"]), after("b", &["a"]), spec("a")]).unwrap();

    finish(&mut graph, "a", Outcome::Failed);

    assert_eq!(state_of(&graph, "b"), TaskState::Blocked);
    assert_eq!(state_of(&graph, "c"), TaskState::Blocked);
}

/// "A blocked task does not recover by itself".
#[test]
fn a_blocked_task_stays_blocked_when_its_siblings_finish() {
    let mut graph =
        TaskGraph::commit(vec![spec("a"), spec("sibling"), after("b", &["a"])]).unwrap();
    finish(&mut graph, "a", Outcome::Failed);
    assert_eq!(state_of(&graph, "b"), TaskState::Blocked);

    finish(&mut graph, "sibling", Outcome::Done);

    assert_eq!(state_of(&graph, "b"), TaskState::Blocked);
}

#[test]
fn a_failure_leaves_tasks_that_do_not_depend_on_it_alone() {
    let mut graph =
        TaskGraph::commit(vec![spec("a"), spec("unrelated"), after("b", &["a"])]).unwrap();

    finish(&mut graph, "a", Outcome::Failed);

    assert_eq!(state_of(&graph, "unrelated"), TaskState::Ready);
}

#[test]
fn an_outcome_can_only_be_reported_for_a_dispatched_task() {
    let mut graph = TaskGraph::commit(vec![spec("a")]).unwrap();

    let error = graph.complete(&id("a"), Outcome::Done).unwrap_err();

    assert_eq!(error,
               TaskError::NotDispatched { task:  id("a"),
                                          state: "ready", });
}

// ---------------------------------------------------------------------------
// The dispatch gate
// ---------------------------------------------------------------------------

/// "An unmet dependency names itself".
#[test]
fn the_gate_names_the_dependencies_that_are_not_done() {
    let mut graph = TaskGraph::commit(vec![spec("a"), spec("b"), after("c", &["a", "b"])]).unwrap();
    graph.mark_dispatched(&id("a"), Uuid::new_v4()).unwrap();

    let error = graph.gate(&id("c")).unwrap_err();

    assert_eq!(error,
               TaskError::DependenciesUnmet { task:  id("c"),
                                              unmet: vec![id("a"), id("b")], });
}

#[test]
fn the_gate_hands_back_what_to_deliver_and_to_whom() {
    let agent = Uuid::new_v4();
    let graph = TaskGraph::commit(vec![spec("a").assigned_to(agent)]).unwrap();

    let dispatchable = graph.gate(&id("a")).unwrap();

    assert_eq!(dispatchable.goal, "do a");
    assert_eq!(dispatchable.assignee, Some(Assignee::Agent(agent)));
}

/// A capability assignee is handed back unresolved: the caller owns the
/// registry, and resolving here would re-point a task on every re-plan.
#[test]
fn the_gate_hands_back_a_capability_assignee_unresolved() {
    let graph =
        TaskGraph::commit(vec![spec("a").for_capabilities(["code-review"].iter().collect())]).unwrap();

    let dispatchable = graph.gate(&id("a")).unwrap();

    match dispatchable.assignee {
        Some(Assignee::Capabilities(tags)) => assert!(tags.contains("code-review")),
        other => panic!("expected an unresolved capability assignee, got {other:?}"),
    }
}

#[test]
fn a_task_may_be_planned_with_no_assignee() {
    let graph = TaskGraph::commit(vec![after("a", &[])]).unwrap();

    assert_eq!(graph.gate(&id("a")).unwrap().assignee, None);
}

#[test]
fn an_unknown_task_cannot_be_dispatched() {
    let graph = TaskGraph::commit(vec![spec("a")]).unwrap();

    assert_eq!(graph.gate(&id("ghost")).unwrap_err(),
               TaskError::NotFound(id("ghost")));
}

#[test]
fn a_dispatched_task_cannot_be_dispatched_again() {
    let mut graph = TaskGraph::commit(vec![spec("a")]).unwrap();
    graph.mark_dispatched(&id("a"), Uuid::new_v4()).unwrap();

    let error = graph.gate(&id("a")).unwrap_err();

    assert_eq!(error,
               TaskError::NotReady { task:  id("a"),
                                     state: "dispatched", });
}

/// "A rejected delivery does not consume the task". The gate reads only;
/// nothing moves until the caller says delivery succeeded.
#[test]
fn gating_alone_does_not_move_a_task() {
    let graph = TaskGraph::commit(vec![spec("a")]).unwrap();

    graph.gate(&id("a")).expect("dispatchable");

    assert_eq!(state_of(&graph, "a"), TaskState::Ready);
}

#[test]
fn a_dispatch_records_the_agent_it_actually_went_to() {
    let chosen = Uuid::new_v4();
    let mut graph =
        TaskGraph::commit(vec![spec("a").for_capabilities(["rust"].iter().collect())]).unwrap();

    graph.mark_dispatched(&id("a"), chosen).unwrap();

    assert_eq!(graph.task(&id("a")).unwrap().dispatched_to, Some(chosen));
}

// ---------------------------------------------------------------------------
// What counts as nontrivial
// ---------------------------------------------------------------------------

#[test]
fn one_task_for_one_agent_is_trivial() {
    let graph = TaskGraph::commit(vec![spec("a").assigned_to(Uuid::new_v4())]).unwrap();

    assert!(!graph.is_nontrivial());
}

#[test]
fn more_than_one_task_is_nontrivial() {
    let graph = TaskGraph::commit(vec![spec("a"), spec("b")]).unwrap();

    assert!(graph.is_nontrivial());
}

#[test]
fn more_than_one_agent_is_nontrivial() {
    let graph = TaskGraph::commit(vec![spec("a").assigned_to(Uuid::new_v4()).after([])]).unwrap();
    assert!(!graph.is_nontrivial(), "one agent, one task");

    let two = TaskGraph::commit(vec![spec("a").assigned_to(Uuid::new_v4()),
                                     spec("b").assigned_to(Uuid::new_v4())]).unwrap();
    assert!(two.is_nontrivial());
}

// ---------------------------------------------------------------------------
// Re-planning
// ---------------------------------------------------------------------------

/// "A dispatched task keeps its state across a re-plan".
#[test]
fn a_dispatched_task_survives_a_replan_unchanged() {
    let agent = Uuid::new_v4();
    let mut graph = TaskGraph::commit(vec![spec("a"), spec("b")]).unwrap();
    graph.mark_dispatched(&id("a"), agent).unwrap();

    let revised = graph.recommit(vec![spec("a"), spec("b"), spec("c")])
                       .unwrap();

    assert_eq!(state_of(&revised, "a"), TaskState::Dispatched);
    assert_eq!(revised.task(&id("a")).unwrap().dispatched_to, Some(agent));
}

/// "A completed task is not re-run".
#[test]
fn a_done_task_survives_a_replan_and_is_not_dispatchable_again() {
    let mut graph = TaskGraph::commit(vec![spec("a")]).unwrap();
    finish(&mut graph, "a", Outcome::Done);

    let revised = graph.recommit(vec![spec("a"), after("b", &["a"])]).unwrap();

    assert_eq!(state_of(&revised, "a"), TaskState::Done);
    assert!(revised.gate(&id("a")).is_err());
    // And the new task behind it is ready at once, since `a` already ran.
    assert_eq!(state_of(&revised, "b"), TaskState::Ready);
}

#[test]
fn a_task_dropped_from_the_revised_plan_is_gone() {
    let graph = TaskGraph::commit(vec![spec("a"), spec("b")]).unwrap();

    let revised = graph.recommit(vec![spec("a")]).unwrap();

    assert!(revised.task(&id("b")).is_none());
    assert_eq!(revised.tasks().len(), 1);
}

#[test]
fn a_replan_may_add_a_dependency_to_an_untouched_task() {
    let graph = TaskGraph::commit(vec![spec("a"), spec("b")]).unwrap();
    assert_eq!(state_of(&graph, "b"), TaskState::Ready);

    let revised = graph.recommit(vec![spec("a"), after("b", &["a"])]).unwrap();

    assert_eq!(state_of(&revised, "b"), TaskState::Pending);
}

#[test]
fn submission_order_is_preserved_for_display() {
    let graph = TaskGraph::commit(vec![spec("zeta"), spec("alpha"), spec("middle")]).unwrap();

    let ids: Vec<&str> = graph.tasks().iter().map(|task| task.id.as_str()).collect();
    assert_eq!(ids, vec!["zeta", "alpha", "middle"]);
}

#[test]
fn an_empty_plan_is_allowed_and_holds_nothing() {
    let graph = TaskGraph::commit(Vec::new()).unwrap();

    assert!(graph.is_empty());
    assert!(!graph.is_nontrivial());
}
