//! The recorded-pull-requests document: that it round-trips, that writing it
//! touches nothing else, and that a store predating it grows no file for a
//! feature it has never used.

use std::fs;

use tempfile::tempdir;

use super::super::*;
use super::{agent_id, other_documents};
use crate::consts::PULL_REQUESTS_FILE;

fn pull_request(url: &str) -> SavedPullRequest {
    SavedPullRequest::new(url, agent_id(), agent_id())
}

#[test]
fn recorded_pull_requests_round_trip_through_their_own_document() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.pull_requests = vec![pull_request("https://github.com/acme/widget/pull/42")];

    s.persist_pull_requests().unwrap();

    let back = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(back.pull_requests, s.pull_requests);
}

/// The reason this is a document of its own rather than a field on
/// `SavedAgent`: a URL scrolling past must not rewrite the agent roster.
#[test]
fn recording_a_pull_request_leaves_every_other_document_untouched() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.recent_repos = vec!["alpha".to_string()];
    s.saved_agents = vec![SavedAgent::new(agent_id(), "A", None, "/tmp")];
    s.persist().unwrap();
    let before = other_documents(&dir, PULL_REQUESTS_FILE);

    s.pull_requests
     .push(pull_request("https://github.com/acme/widget/pull/42"));
    s.persist_pull_requests().unwrap();

    assert_eq!(other_documents(&dir, PULL_REQUESTS_FILE), before);
    assert_eq!(Settings::load_from_root(dir.path()).unwrap()
                                                   .pull_requests
                                                   .len(),
               1);
}

/// A store written before this document existed is every store that exists
/// today. It has to load, and it must not grow a file for a feature it has
/// never used.
#[test]
fn a_store_predating_the_document_loads_with_none_and_writes_none() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.recent_repos = vec!["alpha".to_string()];
    s.persist().unwrap();

    let loaded = Settings::load_from_root(dir.path()).unwrap();

    assert!(loaded.pull_requests.is_empty());
    assert!(!dir.path().join(PULL_REQUESTS_FILE).exists(),
            "no document until one is recorded");
}

/// Once the document exists it is always written, or removing the last row
/// would come back on the next launch.
#[test]
fn removing_the_last_record_still_writes_the_document() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.pull_requests = vec![pull_request("https://github.com/acme/widget/pull/42")];
    s.persist_pull_requests().unwrap();

    s.pull_requests.clear();
    s.persist_pull_requests().unwrap();

    assert!(dir.path().join(PULL_REQUESTS_FILE).exists());
    assert!(Settings::load_from_root(dir.path()).unwrap()
                                                .pull_requests
                                                .is_empty());
}

#[test]
fn a_corrupt_pull_request_document_costs_only_itself() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_root(dir.path());
    s.mcp_server_port = 9400;
    s.recent_repos = vec!["alpha".to_string()];
    s.pull_requests = vec![pull_request("https://github.com/acme/widget/pull/42")];
    s.persist().unwrap();
    fs::write(dir.path().join(PULL_REQUESTS_FILE), "{not json").unwrap();

    let loaded = Settings::load_from_root(dir.path()).unwrap();

    assert!(loaded.pull_requests.is_empty());
    assert_eq!(loaded.mcp_server_port, 9400);
    assert_eq!(loaded.recent_repos, vec!["alpha".to_string()]);
}
