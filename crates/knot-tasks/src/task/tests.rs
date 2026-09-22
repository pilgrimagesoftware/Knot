//! Unit tests for [`super`].

use super::*;

#[test]
fn a_task_with_no_dependencies_starts_ready() {
    let task = Task::from_spec(TaskSpec::new("a", "do the thing"));

    assert_eq!(task.state, TaskState::Ready);
}

#[test]
fn a_task_with_dependencies_starts_pending() {
    let task = Task::from_spec(TaskSpec::new("b", "after a").after([TaskId::new("a")]));

    assert_eq!(task.state, TaskState::Pending);
}

#[test]
fn every_state_round_trips_through_its_string() {
    for state in [TaskState::Pending,
                  TaskState::Ready,
                  TaskState::Dispatched,
                  TaskState::Done,
                  TaskState::Failed,
                  TaskState::Blocked]
    {
        assert_eq!(state.as_str().parse::<TaskState>(), Ok(state));
        assert_eq!(serde_json::to_string(&state).unwrap(),
                   format!("\"{}\"", state.as_str()));
    }
}

#[test]
fn an_unknown_state_is_reported_rather_than_defaulted() {
    assert_eq!("retrying".parse::<TaskState>(),
               Err(UnknownTaskState("retrying".to_string())));
}

/// A readiness pass must never touch a task that is in flight or finished,
/// or a dispatched task would be re-sent and a done one re-run.
#[test]
fn settled_states_are_the_ones_a_readiness_pass_must_not_touch() {
    assert!(!TaskState::Pending.is_settled());
    assert!(!TaskState::Ready.is_settled());
    assert!(TaskState::Dispatched.is_settled());
    assert!(TaskState::Done.is_settled());
    assert!(TaskState::Failed.is_settled());
    assert!(TaskState::Blocked.is_settled());
}
