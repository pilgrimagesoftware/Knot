//! Recording, forgetting and listing the pull requests Knot has seen.

use super::super::*;

const FIRST: &str = "https://github.com/acme/widget/pull/42";
const SECOND: &str = "https://github.com/acme/widget/pull/43";

fn store_with_agent() -> (AgentStore, Uuid) {
    let mut store = AgentStore::new();
    let agent = store.create("/tmp/widget", CreateOptions::default());
    (store, agent)
}

/// An empty workspace with a fresh id, added to `store`.
fn add_workspace(store: &mut AgentStore, name: &str) -> Uuid {
    let workspace = knot_core::Workspace { id:        Uuid::new_v4(),
                                           name:      name.to_string(),
                                           color_hex: "#000000".to_string(),
                                           agent_ids: Vec::new(), };
    let id = workspace.id;
    store.add_workspace(workspace);
    id
}

#[test]
fn recording_a_url_attributes_it_to_the_agent_and_its_workspace() {
    let (mut store, agent) = store_with_agent();
    let workspace = store.workspaces()[0].id;

    assert!(store.record_pull_request(agent, FIRST));

    let records = store.pull_requests();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].url, FIRST);
    assert_eq!(records[0].agent_id, agent);
    assert_eq!(records[0].workspace_id, workspace);
}

/// A URL scrolls past twice, or a tool call is re-rendered. Both taps offer
/// every URL they see without checking first, so the store has to be the one
/// that does.
#[test]
fn recording_the_same_url_again_is_a_no_op() {
    let (mut store, agent) = store_with_agent();
    assert!(store.record_pull_request(agent, FIRST));
    let first_seen = store.pull_requests()[0].first_seen;

    assert!(!store.record_pull_request(agent, FIRST));
    assert!(!store.record_pull_request(agent, FIRST));

    assert_eq!(store.pull_requests().len(), 1);
    assert_eq!(store.pull_requests()[0].first_seen,
               first_seen,
               "re-seeing a URL must not restamp it");
}

/// Who opened it is part of what is recorded, so one pull request seen by two
/// agents is two records.
#[test]
fn the_same_url_from_two_agents_is_two_records() {
    let (mut store, first_agent) = store_with_agent();
    let second_agent = store.create("/tmp/widget", CreateOptions::default());

    assert!(store.record_pull_request(first_agent, FIRST));
    assert!(store.record_pull_request(second_agent, FIRST));

    assert_eq!(store.pull_requests().len(), 2);
    assert_eq!(store.pull_requests_for_agent(first_agent).len(), 1);
    assert_eq!(store.pull_requests_for_agent(second_agent).len(), 1);
}

/// An agent can be removed while output about it is still in flight.
#[test]
fn recording_against_an_unknown_agent_records_nothing() {
    let (mut store, _) = store_with_agent();

    assert!(!store.record_pull_request(Uuid::new_v4(), FIRST));

    assert!(store.pull_requests().is_empty());
}

#[test]
fn removing_a_record_affects_only_that_agents_copy() {
    let (mut store, first_agent) = store_with_agent();
    let second_agent = store.create("/tmp/widget", CreateOptions::default());
    store.record_pull_request(first_agent, FIRST);
    store.record_pull_request(second_agent, FIRST);

    assert!(store.remove_pull_request(first_agent, FIRST));

    assert_eq!(store.pull_requests().len(), 1);
    assert_eq!(store.pull_requests()[0].agent_id, second_agent);
    assert!(!store.remove_pull_request(first_agent, FIRST),
            "already gone");
}

/// Knot records what it sees. Having seen it again is a new sighting.
#[test]
fn a_removed_url_is_recorded_again_if_it_is_seen_again() {
    let (mut store, agent) = store_with_agent();
    store.record_pull_request(agent, FIRST);
    store.remove_pull_request(agent, FIRST);

    assert!(store.record_pull_request(agent, FIRST));

    assert_eq!(store.pull_requests().len(), 1);
}

// --- Listing ----------------------------------------------------------------

#[test]
fn a_workspaces_records_list_newest_first() {
    let (mut store, agent) = store_with_agent();
    let workspace = store.workspaces()[0].id;
    store.record_pull_request(agent, FIRST);
    store.record_pull_request(agent, SECOND);
    // Stamped within the same second in a test; force an order.
    store.set_pull_requests({
             let mut records = store.pull_requests().to_vec();
             records[0].first_seen = 100;
             records[1].first_seen = 200;
             records
         });

    let listed = store.pull_requests_for_workspace(workspace);

    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].url, SECOND, "newest first");
    assert_eq!(listed[1].url, FIRST);
}

/// Records stamped in the same second must not shuffle between frames.
#[test]
fn records_seen_in_the_same_second_list_in_a_stable_order() {
    let (mut store, agent) = store_with_agent();
    let workspace = store.workspaces()[0].id;
    store.record_pull_request(agent, SECOND);
    store.record_pull_request(agent, FIRST);

    let once = store.pull_requests_for_workspace(workspace)
                    .iter()
                    .map(|record| record.url.clone())
                    .collect::<Vec<_>>();
    let twice = store.pull_requests_for_workspace(workspace)
                     .iter()
                     .map(|record| record.url.clone())
                     .collect::<Vec<_>>();

    assert_eq!(once, twice);
}

#[test]
fn another_workspaces_records_are_not_listed() {
    let (mut store, agent) = store_with_agent();
    let other_workspace = add_workspace(&mut store, "Other");
    let other_agent = store.create("/tmp/other",
                                   CreateOptions { workspace_id: Some(other_workspace),
                                                   ..CreateOptions::default() });
    store.record_pull_request(agent, FIRST);
    store.record_pull_request(other_agent, SECOND);

    let listed = store.pull_requests_for_workspace(other_workspace);

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].url, SECOND);
}

// --- Cascades ---------------------------------------------------------------

/// A record with no agent has nothing to show it under.
#[test]
fn removing_an_agent_drops_its_records() {
    let (mut store, agent) = store_with_agent();
    let other = store.create("/tmp/widget", CreateOptions::default());
    store.record_pull_request(agent, FIRST);
    store.record_pull_request(other, SECOND);

    store.remove(agent);

    assert_eq!(store.pull_requests().len(), 1);
    assert_eq!(store.pull_requests()[0].agent_id, other);
}

#[test]
fn removing_a_workspace_drops_its_agents_records() {
    let (mut store, agent) = store_with_agent();
    let other_workspace = add_workspace(&mut store, "Other");
    let other_agent = store.create("/tmp/other",
                                   CreateOptions { workspace_id: Some(other_workspace),
                                                   ..CreateOptions::default() });
    store.record_pull_request(agent, FIRST);
    store.record_pull_request(other_agent, SECOND);

    store.remove_workspace(other_workspace);

    assert_eq!(store.pull_requests().len(), 1);
    assert_eq!(store.pull_requests()[0].agent_id, agent);
}

/// The case removing the agents does not cover: a record naming a workspace
/// its agent has since left would otherwise point at a workspace that is
/// gone.
#[test]
fn removing_a_workspace_drops_records_whose_agent_has_moved_away() {
    let (mut store, agent) = store_with_agent();
    let first_workspace = store.workspaces()[0].id;
    let second_workspace = add_workspace(&mut store, "Second");
    store.record_pull_request(agent, FIRST);
    store.move_to_workspace(agent, second_workspace);

    store.remove_workspace(first_workspace);

    assert!(store.pull_requests().is_empty());
}

/// The two detection taps meet here. An agent whose URL reaches the store
/// from both the panel buffer and the terminal buffer - the same poll tick
/// draining both - must still be one record, because "a panel agent opened a
/// pull request" and "a terminal agent opened one" are the same event to the
/// user.
#[test]
fn the_same_url_from_both_taps_is_one_record() {
    let (mut store, agent) = store_with_agent();

    // What the window does with what it drained, in one tick.
    let drained = [(agent, FIRST), (agent, FIRST)];
    let recorded = drained.into_iter()
                          .filter(|(id, url)| store.record_pull_request(*id, *url))
                          .count();

    assert_eq!(recorded, 1);
    assert_eq!(store.pull_requests().len(), 1);
}
