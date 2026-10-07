//! When an agent last went idle: the time the sidebar's status dot reports
//! as its last message (#582).

use std::time::SystemTime;

use super::super::*;

fn running(store: &mut AgentStore) -> Uuid {
    let id = store.create("/repo", CreateOptions::default());
    store.set_state(id, AgentState::Running);
    id
}

#[test]
fn a_new_agent_has_no_last_message() {
    let mut store = AgentStore::new();
    let id = store.create("/repo", CreateOptions::default());
    assert_eq!(store.agent(id).unwrap().idle_since, None);
}

#[test]
fn going_idle_records_when() {
    let mut store = AgentStore::new();
    let id = running(&mut store);
    assert_eq!(store.agent(id).unwrap().idle_since,
               None,
               "starting to work is not a finished message");

    let before = SystemTime::now();
    store.set_state(id, AgentState::Idle);
    let after = SystemTime::now();

    let at = store.agent(id)
                  .unwrap()
                  .idle_since
                  .expect("going idle records the time");
    assert!(before <= at && at <= after);
}

/// Hooks re-report `Idle`; only the transition is the end of a message.
#[test]
fn a_repeated_idle_keeps_the_first_time() {
    let mut store = AgentStore::new();
    let id = running(&mut store);
    store.set_state(id, AgentState::Idle);
    let first = store.agent(id).unwrap().idle_since;

    store.set_state(id, AgentState::Idle);

    assert_eq!(store.agent(id).unwrap().idle_since, first);
}

/// The time survives the agent working again, so the next idle replaces it
/// rather than the dot ever showing a gap.
#[test]
fn the_next_idle_replaces_it() {
    let mut store = AgentStore::new();
    let id = running(&mut store);
    store.set_state(id, AgentState::Idle);
    let first = store.agent(id).unwrap().idle_since.unwrap();

    store.set_state(id, AgentState::Running);
    assert_eq!(store.agent(id).unwrap().idle_since, Some(first));
    std::thread::sleep(std::time::Duration::from_millis(2));
    store.set_state(id, AgentState::Idle);

    assert!(store.agent(id).unwrap().idle_since.unwrap() > first);
}

/// A restart starts a new session, which has said nothing yet.
#[test]
fn a_restart_clears_it() {
    let mut store = AgentStore::new();
    let id = running(&mut store);
    store.set_state(id, AgentState::Idle);

    store.restart(id).expect("restart succeeds");

    assert_eq!(store.agent(id).unwrap().idle_since, None);
}
