//! Which of a workspace's pull request rows the Pull Requests view draws, and
//! in what order: the search, the status and agent filters, and the sort.
//!
//! Contract: "The user can search / filter / sort the Pull Requests view" in
//! the `pull-request-tracking` capability spec under `openspec/specs/`.
//!
//! Pure, over rows already flattened out of the store and the state cache, so
//! every rule here is testable without GPUI or a lock. The window half is
//! `workspace_window::pull_requests_view`; this decides nothing about what is
//! fetched - a row hidden here is still refreshed, because the launcher row
//! counts it and expiry depends on it.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use knot_forge::{Mergeability, PullRequestStatus};
use uuid::Uuid;

use crate::pull_request_state::PullRequestLookup;

/// One row, flattened out of the store and the state cache before any
/// element is built - so neither lock is held across the element tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PullRequestRow {
    pub(crate) url:        String,
    /// `None` while nothing has been asked for it yet.
    pub(crate) lookup:     Option<PullRequestLookup>,
    /// The earliest time any of the row's agents first saw it, in Unix
    /// seconds. What newest and oldest first order by.
    pub(crate) first_seen: i64,
}

/// The rows one set of agents opened, under their names.
///
/// Several agents rather than one: a pull request several agents recorded is
/// one row, headed by all of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PullRequestGroup {
    /// Every agent each row here is attributed to, so removing a row
    /// addresses every record behind it.
    pub(crate) agent_ids: Vec<Uuid>,
    /// Their names, joined for the heading.
    pub(crate) agents:    String,
    pub(crate) rows:      Vec<PullRequestRow>,
}

/// Where a row stands, in the launcher row's vocabulary.
///
/// The one classification the launcher counts, the status toggles and the
/// bulk removals all read, so "3 open" in the sidebar and "Open 3" on the
/// toggle cannot disagree. Draft is open, as the forge models it; a failed
/// fetch is pending, since it is retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RowCategory {
    Open,
    Merged,
    Closed,
    NotFound,
    Pending,
}

impl RowCategory {
    /// Every category, in the order the toggles show them.
    pub(crate) const ALL: [Self; 5] = [Self::Open,
                                       Self::Merged,
                                       Self::Closed,
                                       Self::NotFound,
                                       Self::Pending];

    pub(crate) fn of(lookup: Option<&PullRequestLookup>) -> Self {
        match lookup {
            Some(PullRequestLookup::Known(state)) => match state.status {
                PullRequestStatus::Merged => Self::Merged,
                PullRequestStatus::Closed => Self::Closed,
                PullRequestStatus::Draft | PullRequestStatus::Open => Self::Open,
            },
            Some(PullRequestLookup::NotFound) => Self::NotFound,
            Some(PullRequestLookup::Failed) | None => Self::Pending,
        }
    }
}

impl fmt::Display for RowCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self {
            Self::Open => "pull_requests.open",
            Self::Merged => "pull_requests.merged",
            Self::Closed => "pull_requests.closed",
            Self::NotFound => "pull_requests.not_found",
            Self::Pending => "pull_requests.filter.pending",
        };
        f.write_str(&knot_core::l10n::t(key))
    }
}

/// The order rows take within each group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PullRequestSort {
    #[default]
    NewestFirst,
    OldestFirst,
    NeedsAttention,
    Repository,
}

impl PullRequestSort {
    /// Every order, in the order the picker lists them.
    pub(crate) const ALL: [Self; 4] = [Self::NewestFirst,
                                       Self::OldestFirst,
                                       Self::NeedsAttention,
                                       Self::Repository];

    fn compare(self, left: &PullRequestRow, right: &PullRequestRow) -> Ordering {
        let primary = match self {
            Self::NewestFirst => Ordering::Equal,
            Self::OldestFirst => left.first_seen.cmp(&right.first_seen),
            Self::NeedsAttention => attention_rank(left).cmp(&attention_rank(right)),
            Self::Repository => repository_key(left).cmp(&repository_key(right)),
        };
        // Newest first, then the URL, under every order: the order has to be
        // total, or two rows seen in the same second swap between frames.
        primary.then_with(|| right.first_seen.cmp(&left.first_seen))
               .then_with(|| left.url.cmp(&right.url))
    }
}

impl fmt::Display for PullRequestSort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self {
            Self::NewestFirst => "pull_requests.sort.newest",
            Self::OldestFirst => "pull_requests.sort.oldest",
            Self::NeedsAttention => "pull_requests.sort.attention",
            Self::Repository => "pull_requests.sort.repository",
        };
        f.write_str(&knot_core::l10n::t(key))
    }
}

/// What narrows the list: the search text, the selected status toggles
/// (none selected shows every status) and the chosen agent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ViewFilter {
    pub(crate) search:   String,
    pub(crate) statuses: BTreeSet<RowCategory>,
    pub(crate) agent:    Option<Uuid>,
}

impl ViewFilter {
    /// Whether anything narrows the list, so a bulk action says it acts on
    /// the shown rows only.
    pub(crate) fn is_active(&self) -> bool {
        !self.search.trim().is_empty() || !self.statuses.is_empty() || self.agent.is_some()
    }

    /// The search and the agent filter, without the status filter: what a
    /// toggle's count is taken over.
    fn passes_search_and_agent(&self, needle: &str, group: &PullRequestGroup,
                               row: &PullRequestRow)
                               -> bool {
        let agent_ok = self.agent
                           .is_none_or(|agent| group.agent_ids.contains(&agent));
        agent_ok && (needle.is_empty() || row_matches(needle, group, row))
    }

    fn passes_status(&self, row: &PullRequestRow) -> bool {
        self.statuses.is_empty()
        || self.statuses
               .contains(&RowCategory::of(row.lookup.as_ref()))
    }
}

/// The search text as it is compared: trimmed and lowercased once, rather
/// than once per row.
fn needle(search: &str) -> String {
    search.trim().to_lowercase()
}

/// Whether a row carries `needle` in its title, `#number`, `owner/repo`, URL
/// or agent names. A row with no state is matched on what its URL says, so a
/// search does not hide it for want of a title that has not arrived.
fn row_matches(needle: &str, group: &PullRequestGroup, row: &PullRequestRow) -> bool {
    let state = row.lookup.as_ref().and_then(PullRequestLookup::state);
    let parsed = repo_and_number(&row.url);
    let number = state.and_then(|state| state.number)
                      .or(parsed.as_ref().map(|(_, number)| *number));
    let title = state.and_then(|state| state.title.as_deref());
    let haystacks = [title.map(str::to_lowercase),
                     number.map(|number| format!("#{number}")),
                     parsed.map(|(repo, _)| repo.to_lowercase()),
                     Some(row.url.to_lowercase()),
                     Some(group.agents.to_lowercase())];
    haystacks.iter()
             .flatten()
             .any(|haystack| haystack.contains(needle))
}

/// `owner/repo` and the number from a pull request URL, on any host.
///
/// Parsed here rather than through `knot_core`'s scanner, which only knows
/// the hosts it recognises by default: a record from a GitHub Enterprise host
/// is still a recorded pull request and still sorts by repository.
pub(crate) fn repo_and_number(url: &str) -> Option<(String, u64)> {
    let path = url.split_once("://").map_or(url, |(_, rest)| rest);
    let mut segments = path.split('/');
    let _host = segments.next()?;
    let owner = segments.next()?;
    let repo = segments.next()?;
    if segments.next()? != "pull" {
        return None;
    }
    let number = segments.next()?.parse().ok()?;
    Some((format!("{owner}/{repo}"), number))
}

/// The repository order's key. A URL that does not parse sorts after every
/// one that does, by its URL.
fn repository_key(row: &PullRequestRow) -> (bool, String, u64) {
    match repo_and_number(&row.url) {
        Some((repo, number)) => (false, repo.to_lowercase(), number),
        None => (true, row.url.clone(), 0),
    }
}

/// Where a row falls in the needs-attention order: open rows that cannot land
/// first, in the row colours' precedence, then the rest by how little they
/// ask of the user.
pub(crate) fn attention_rank(row: &PullRequestRow) -> u8 {
    match row.lookup.as_ref() {
        Some(PullRequestLookup::Known(state)) => match state.status {
            PullRequestStatus::Draft | PullRequestStatus::Open => match state.mergeable {
                Mergeability::Conflicting | Mergeability::Blocked => 0,
                Mergeability::Behind => 1,
                Mergeability::ChecksRunning => 2,
                Mergeability::Unknown => 3,
                Mergeability::Mergeable => 4,
            },
            PullRequestStatus::Closed => 7,
            PullRequestStatus::Merged => 8,
        },
        Some(PullRequestLookup::Failed) | None => 5,
        Some(PullRequestLookup::NotFound) => 6,
    }
}

/// Every agent that heads at least one group.
///
/// What the agent picker offers, and what decides whether a chosen agent is
/// still worth filtering on.
pub(crate) fn listed_agents(groups: &[PullRequestGroup]) -> BTreeSet<Uuid> {
    groups.iter()
          .flat_map(|group| group.agent_ids.iter().copied())
          .collect()
}

/// The groups the view draws: rows that pass the search and both filters,
/// sorted within each group, with groups left empty dropped. Group order is
/// untouched.
pub(crate) fn apply(groups: Vec<PullRequestGroup>, filter: &ViewFilter, sort: PullRequestSort)
                    -> Vec<PullRequestGroup> {
    let needle = needle(&filter.search);
    groups.into_iter()
          .filter_map(|mut group| {
              let mut rows = std::mem::take(&mut group.rows);
              rows.retain(|row| {
                      filter.passes_search_and_agent(&needle, &group, row)
                      && filter.passes_status(row)
                  });
              if rows.is_empty() {
                  return None;
              }
              rows.sort_by(|left, right| sort.compare(left, right));
              group.rows = rows;
              Some(group)
          })
          .collect()
}

/// How many rows each status toggle would show, over the rows the search and
/// the agent filter leave. The status filter itself is ignored: a toggle's
/// number says what choosing it would add.
pub(crate) fn category_counts(groups: &[PullRequestGroup], filter: &ViewFilter)
                              -> BTreeMap<RowCategory, usize> {
    let needle = needle(&filter.search);
    let mut counts = BTreeMap::new();
    for group in groups {
        for row in &group.rows {
            if filter.passes_search_and_agent(&needle, group, row) {
                *counts.entry(RowCategory::of(row.lookup.as_ref()))
                       .or_insert(0) += 1;
            }
        }
    }
    counts
}

/// The shown rows a bulk action acts on, each with every agent it is
/// attributed to, in the order they are drawn. `only` narrows to one status,
/// for Remove merged / closed / not found.
pub(crate) fn shown_rows(shown: &[PullRequestGroup], only: Option<RowCategory>)
                         -> Vec<(Vec<Uuid>, String)> {
    shown.iter()
         .flat_map(|group| {
             group.rows
                  .iter()
                  .filter(move |row| {
                      only.is_none_or(|category| RowCategory::of(row.lookup.as_ref()) == category)
                  })
                  .map(|row| (group.agent_ids.clone(), row.url.clone()))
         })
         .collect()
}

/// The shown URLs, one per line, in the order they are drawn.
pub(crate) fn copied_urls(shown: &[PullRequestGroup]) -> String {
    shown_rows(shown, None).into_iter()
                           .map(|(_, url)| url)
                           .collect::<Vec<_>>()
                           .join("\n")
}

#[cfg(test)]
mod tests;
