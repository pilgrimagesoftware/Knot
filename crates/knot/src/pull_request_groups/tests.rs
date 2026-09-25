//! Unit tests for [`super`]: which row goes in which group, in which order,
//! and what the launcher row counts.

use std::collections::BTreeMap;

use knot_core::SavedPullRequest;
use uuid::Uuid;

use super::{OwnedPullRequests, first_seen_by_url, group_records, unique_urls};
use crate::pull_request_state::counts_for;

const FIRST: &str = "https://github.com/acme/widget/pull/1";
const SECOND: &str = "https://github.com/acme/widget/pull/2";
const THIRD: &str = "https://github.com/acme/widget/pull/3";

fn record(url: &str, agent_id: Uuid, first_seen: i64) -> SavedPullRequest {
    SavedPullRequest { first_seen,
                       ..SavedPullRequest::new(url, agent_id, Uuid::nil()) }
}

fn sorted(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort_unstable();
    ids
}

#[test]
fn a_pull_request_one_agent_recorded_stays_in_its_group() {
    let agent = Uuid::new_v4();
    let records = [record(FIRST, agent, 10)];

    let groups = group_records(&records.iter().collect::<Vec<_>>());

    assert_eq!(groups,
               vec![OwnedPullRequests { owners: vec![agent],
                                        urls:   vec![FIRST.to_string()], }]);
}

/// The defect in #444: one pull request, recorded by two agents, drawn twice.
#[test]
fn a_pull_request_two_agents_recorded_is_one_row_under_both() {
    let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
    let records = [record(FIRST, one, 10), record(FIRST, two, 20)];

    let groups = group_records(&records.iter().collect::<Vec<_>>());

    assert_eq!(groups,
               vec![OwnedPullRequests { owners: sorted(vec![one, two]),
                                        urls:   vec![FIRST.to_string()], }]);
}

#[test]
fn shared_groups_come_before_single_agent_ones() {
    let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
    // The single-agent row is the newest, so only the tier puts shared first.
    let records = [record(SECOND, one, 99),
                   record(FIRST, one, 10),
                   record(FIRST, two, 10)];

    let groups = group_records(&records.iter().collect::<Vec<_>>());

    assert_eq!(groups.len(), 2);
    assert!(groups[0].is_shared());
    assert_eq!(groups[1].urls, vec![SECOND.to_string()]);
}

/// A shared row orders by the first time anyone saw it, so a second agent
/// linking an old pull request does not lift it to the top.
#[test]
fn a_shared_row_orders_by_its_earliest_sighting() {
    let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
    let records = [record(FIRST, one, 10),
                   record(FIRST, two, 90),
                   record(SECOND, one, 50),
                   record(SECOND, two, 50)];

    let groups = group_records(&records.iter().collect::<Vec<_>>());

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].urls, vec![SECOND.to_string(), FIRST.to_string()]);
}

/// Grouping is by the whole set of agents, so every heading is true of every
/// row beneath it.
#[test]
fn only_pull_requests_with_the_same_agents_share_a_group() {
    let (one, two, three) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let records = [record(FIRST, one, 30),
                   record(FIRST, two, 30),
                   record(SECOND, two, 20),
                   record(SECOND, one, 20),
                   record(THIRD, two, 10),
                   record(THIRD, three, 10)];

    let groups = group_records(&records.iter().collect::<Vec<_>>());

    assert_eq!(groups,
               vec![OwnedPullRequests { owners: sorted(vec![one, two]),
                                        urls:   vec![FIRST.to_string(), SECOND.to_string()], },
                    OwnedPullRequests { owners: sorted(vec![two, three]),
                                        urls:   vec![THIRD.to_string()], }]);
}

/// The launcher row half of #444: six agents on one pull request counted six
/// open pull requests.
#[test]
fn a_pull_request_several_agents_recorded_counts_once() {
    let records = (0..6).map(|_| record(FIRST, Uuid::new_v4(), 10))
                        .chain([record(SECOND, Uuid::new_v4(), 5)])
                        .collect::<Vec<_>>();

    let urls = unique_urls(&records.iter().collect::<Vec<_>>());

    assert_eq!(urls, vec![FIRST.to_string(), SECOND.to_string()]);
    assert_eq!(counts_for(&BTreeMap::new(), &urls).total(), 2);
}

/// A row several agents saw is as old as the first sighting.
#[test]
fn first_seen_is_the_earliest_across_agents() {
    let (one, two) = (Uuid::new_v4(), Uuid::new_v4());
    let early = record(FIRST, one, 100);
    let late = record(FIRST, two, 300);

    let seen = first_seen_by_url(&[&late, &early]);

    assert_eq!(seen.get(FIRST), Some(&100));
}
