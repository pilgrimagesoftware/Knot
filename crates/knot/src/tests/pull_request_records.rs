//! Recorded pull requests across a relaunch: what comes back, what stays
//! gone, and what is deliberately not remembered.
//!
//! The window itself needs GPUI, so what is exercised here is the seam it
//! sits on - the settings document, `build_agent_store`, and the store's own
//! mutators - which is where "it came back after a restart" is actually
//! decided.

use tempfile::tempdir;
use uuid::Uuid;

use crate::app_state::build_agent_store;
use crate::tests::workspace;

const FIRST: &str = "https://github.com/acme/widget/pull/42";
const SECOND: &str = "https://github.com/acme/widget/pull/43";

/// A store as the app builds it at launch, with one agent in one workspace.
fn settings_with_agent(root: &std::path::Path) -> (knot_core::Settings, Uuid, Uuid) {
    let agent_id = Uuid::new_v4();
    let mut ws = workspace("Restored");
    ws.agent_ids = vec![agent_id];
    let workspace_id = ws.id;

    let mut settings = knot_core::Settings::with_store_root(root);
    settings.restore_layout_on_launch = true;
    settings.saved_agents = vec![knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha")];
    settings.saved_workspaces = vec![ws];
    (settings, agent_id, workspace_id)
}

/// Writes `settings`, then loads the documents back the way a relaunch does.
fn relaunch(settings: &knot_core::Settings, root: &std::path::Path) -> knot_core::Settings {
    settings.persist().expect("persisted");
    knot_core::Settings::load_from_root(root).expect("loaded")
}

#[test]
fn a_recorded_pull_request_comes_back_after_a_relaunch() {
    let dir = tempdir().unwrap();
    let (mut settings, agent_id, workspace_id) = settings_with_agent(dir.path());
    let mut store = build_agent_store(&settings);
    store.set_pull_requests(settings.pull_requests.clone());
    store.record_pull_request(agent_id, FIRST);
    settings.pull_requests = store.pull_requests().to_vec();

    let reloaded = relaunch(&settings, dir.path());
    let restored = {
        let mut restored = build_agent_store(&reloaded);
        restored.set_pull_requests(reloaded.pull_requests.clone());
        restored
    };

    let listed = restored.pull_requests_for_workspace(workspace_id);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].url, FIRST);
}

/// Task 6.6: a removed row stays removed.
#[test]
fn a_removed_pull_request_stays_absent_after_a_relaunch() {
    let dir = tempdir().unwrap();
    let (mut settings, agent_id, workspace_id) = settings_with_agent(dir.path());
    let mut store = build_agent_store(&settings);
    store.record_pull_request(agent_id, FIRST);
    store.record_pull_request(agent_id, SECOND);
    settings.pull_requests = store.pull_requests().to_vec();
    settings.persist().expect("persisted");

    assert!(store.remove_pull_request(agent_id, FIRST));
    settings.pull_requests = store.pull_requests().to_vec();

    let reloaded = relaunch(&settings, dir.path());
    let mut restored = build_agent_store(&reloaded);
    restored.set_pull_requests(reloaded.pull_requests.clone());

    let listed = restored.pull_requests_for_workspace(workspace_id);
    assert_eq!(listed.len(), 1, "the removed one must not come back");
    assert_eq!(listed[0].url, SECOND);
}

/// Knot records what it sees, so seeing it again after a removal records it
/// again - and that survives the next relaunch too.
#[test]
fn a_removed_pull_request_returns_if_it_is_seen_again() {
    let dir = tempdir().unwrap();
    let (mut settings, agent_id, workspace_id) = settings_with_agent(dir.path());
    let mut store = build_agent_store(&settings);
    store.record_pull_request(agent_id, FIRST);
    store.remove_pull_request(agent_id, FIRST);

    assert!(store.record_pull_request(agent_id, FIRST),
            "seen again is new again");
    settings.pull_requests = store.pull_requests().to_vec();

    let reloaded = relaunch(&settings, dir.path());
    let mut restored = build_agent_store(&reloaded);
    restored.set_pull_requests(reloaded.pull_requests.clone());

    assert_eq!(restored.pull_requests_for_workspace(workspace_id).len(), 1);
}

/// The spec's "a restart shows no stale status": nothing about a pull
/// request's state reaches the document, so there is nothing stale to show.
#[test]
fn a_relaunch_carries_no_fetched_state() {
    let dir = tempdir().unwrap();
    let (mut settings, agent_id, _) = settings_with_agent(dir.path());
    let mut store = build_agent_store(&settings);
    store.record_pull_request(agent_id, FIRST);
    settings.pull_requests = store.pull_requests().to_vec();
    settings.persist().expect("persisted");

    let document =
        std::fs::read_to_string(dir.path().join(knot_core::consts::PULL_REQUESTS_FILE)).unwrap();

    for absent in ["title",
                   "number",
                   "state",
                   "status",
                   "isDraft",
                   "statusCheckRollup"]
    {
        assert!(!document.contains(absent),
                "{absent} was persisted: {document}");
    }
}

/// A record with no agent has nothing to show it under, so a roster that is
/// not being restored takes its records with it.
#[test]
fn records_do_not_outlive_a_roster_that_is_not_restored() {
    let dir = tempdir().unwrap();
    let (mut settings, agent_id, _) = settings_with_agent(dir.path());
    let mut store = build_agent_store(&settings);
    store.record_pull_request(agent_id, FIRST);
    settings.pull_requests = store.pull_requests().to_vec();

    let mut reloaded = relaunch(&settings, dir.path());
    reloaded.restore_layout_on_launch = false;

    assert!(build_agent_store(&reloaded).pull_requests().is_empty());
}

/// The sidebar row counts a workspace's records; the pane draws them under
/// the agent that opened each one, and can draw nothing for an agent that is
/// gone. So a record whose agent left the roster between launches - a
/// document restored on its own, a roster lost - would be a count over an
/// empty pane. It does not survive the launch that finds it.
#[test]
fn records_do_not_outlive_an_agent_missing_from_the_restored_roster() {
    let dir = tempdir().unwrap();
    let (mut settings, agent_id, workspace_id) = settings_with_agent(dir.path());
    let mut store = build_agent_store(&settings);
    store.record_pull_request(agent_id, FIRST);
    settings.pull_requests = store.pull_requests().to_vec();

    let mut reloaded = relaunch(&settings, dir.path());
    reloaded.saved_agents.clear();
    let mut restored = build_agent_store(&reloaded);
    restored.set_pull_requests(reloaded.pull_requests.clone());

    assert!(restored.pull_requests_for_workspace(workspace_id)
                    .is_empty());
}
