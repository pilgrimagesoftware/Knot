//! How a workspace's pull request records are read into the Pull Requests
//! view: one row per pull request, however many agents recorded it.
//!
//! Contract: "The workspace window lists its pull requests" in the
//! `pull-request-tracking` capability spec under `openspec/specs/`.
//!
//! Recording is per agent on purpose - who opened a pull request is part of
//! what is recorded - so a pull request six agents linked is six records.
//! Reading them per record is what listed it six times and counted it six
//! times; this is where they collapse. Pure, so the decision is testable
//! without the store lock or GPUI around it: the view filters out records
//! whose agent has gone before calling in, and joins names and states after.

use knot_core::SavedPullRequest;
use uuid::Uuid;

/// The URLs one set of agents recorded, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedPullRequests {
    /// Every agent that recorded each of these URLs, ordered by id so the set
    /// compares as a value. The view orders them for display.
    pub(crate) owners: Vec<Uuid>,
    pub(crate) urls:   Vec<String>,
}

impl OwnedPullRequests {
    /// Whether several agents recorded these, so the group is drawn first.
    pub(crate) fn is_shared(&self) -> bool {
        self.owners.len() > 1
    }
}

/// Each recorded URL once, in the order `records` carries them.
///
/// What the launcher row counts and the refresh loop walks: a pull request
/// several agents recorded is still one pull request.
pub(crate) fn unique_urls(records: &[&SavedPullRequest]) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for record in records {
        if !urls.contains(&record.url) {
            urls.push(record.url.clone());
        }
    }
    urls
}

/// Group `records` by the set of agents that recorded each URL.
///
/// Keyed on the whole owner set rather than one "shared" bucket, so every
/// heading is true of every row under it: a URL agents A and B recorded and
/// one B and C recorded are two groups. Shared groups come first; within each
/// tier the group holding the newest row leads, as it did when groups were
/// per agent. A row shared by several agents takes the earliest time any of
/// them saw it - the pull request existed from then, and taking the latest
/// would lift it to the top every time another agent linked it.
pub(crate) fn group_records(records: &[&SavedPullRequest]) -> Vec<OwnedPullRequests> {
    let mut pulls: Vec<(&str, Vec<Uuid>, i64)> = Vec::new();
    for record in records {
        match pulls.iter_mut().find(|(url, ..)| *url == record.url) {
            Some((_, owners, first_seen)) => {
                if !owners.contains(&record.agent_id) {
                    owners.push(record.agent_id);
                }
                *first_seen = (*first_seen).min(record.first_seen);
            }
            None => pulls.push((&record.url, vec![record.agent_id], record.first_seen)),
        }
    }
    for (_, owners, _) in &mut pulls {
        owners.sort_unstable();
    }
    // The store's tie-break, so rows seen within one second keep a stable
    // order between frames.
    pulls.sort_by(|left, right| right.2.cmp(&left.2).then_with(|| left.0.cmp(right.0)));

    let mut groups: Vec<OwnedPullRequests> = Vec::new();
    for (url, owners, _) in pulls {
        match groups.iter_mut().find(|group| group.owners == owners) {
            Some(group) => group.urls.push(url.to_string()),
            None => groups.push(OwnedPullRequests { owners,
                                                    urls: vec![url.to_string()] }),
        }
    }
    // Stable, so each tier keeps its newest-row-first order.
    groups.sort_by_key(|group| !group.is_shared());
    groups
}

#[cfg(test)]
mod tests;
