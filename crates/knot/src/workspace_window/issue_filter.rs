//! Which of a workspace's open issues the Issues tab draws, and in what
//! order: the search, the repository filter, the sort, and grouping by
//! repository.
//!
//! Contract: "The user can search, filter and sort the Issues tab" and "The
//! Issues tab lists issues by repository" in
//! `openspec/changes/pr-view-issues-and-changes/specs/workspace-issues/spec.
//! md`.
//!
//! Pure, over issues already fetched per repository, so every rule here is
//! testable without GPUI, a lock or `gh`.

use std::cmp::Ordering;

use knot_forge::{Issue, RepoSlug};

/// One repository's open issues, before search, filter or sort narrow them.
#[derive(Debug, Clone, PartialEq, Eq)]
// UNWIRED(#504): built ahead of the Issues tab render surface. Nothing
// constructs this outside tests; the integrating session wires it into
// `render/issues_pane.rs` alongside the issue cache.
#[allow(dead_code)]
pub(super) struct IssueGroup {
    pub(super) slug:   RepoSlug,
    pub(super) issues: Vec<Issue>,
}

/// The order issues take within each repository group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
// UNWIRED(#504): nothing reads this outside tests until the sort picker in
// `render/issues_toolbar.rs` exists.
#[allow(dead_code)]
pub(super) enum IssueSort {
    /// Most recently updated first. The default: what the tab shows before
    /// the user picks a different order.
    #[default]
    RecentlyUpdated,
    /// Most recently opened first.
    Newest,
    /// Oldest-opened first.
    Oldest,
    /// Ascending issue number.
    Number,
}

// UNWIRED(#504): nothing calls this outside tests until the sort picker in
// `render/issues_toolbar.rs` exists.
#[allow(dead_code)]
impl IssueSort {
    /// Every order, in the order the sort picker lists them.
    pub(super) const ALL: [Self; 4] = [Self::RecentlyUpdated,
                                       Self::Newest,
                                       Self::Oldest,
                                       Self::Number];

    /// Orders two issues already known to share a group. Ties break on
    /// number, ascending, so the order is total and two issues updated in
    /// the same second do not swap between frames.
    fn compare(self, left: &Issue, right: &Issue) -> Ordering {
        let primary = match self {
            Self::RecentlyUpdated => right.updated_at.cmp(&left.updated_at),
            Self::Newest => right.created_at.cmp(&left.created_at),
            Self::Oldest => left.created_at.cmp(&right.created_at),
            Self::Number => left.number.cmp(&right.number),
        };
        primary.then_with(|| left.number.cmp(&right.number))
    }
}

/// What narrows the list: the search text and the chosen repository.
/// `None` shows every workspace repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
// UNWIRED(#504): held by `IssuesViewState` once the integrating session adds
// it; nothing constructs this outside tests yet.
#[allow(dead_code)]
pub(super) struct IssueFilter {
    pub(super) search: String,
    pub(super) repo:   Option<RepoSlug>,
}

// UNWIRED(#504): nothing calls this outside tests until the toolbar in
// `render/issues_toolbar.rs` exists.
#[allow(dead_code)]
impl IssueFilter {
    /// Whether anything narrows the list, so the toolbar knows whether to
    /// offer a control that clears both.
    pub(super) fn is_active(&self) -> bool {
        !self.search.trim().is_empty() || self.repo.is_some()
    }

    /// Return the picker to "All repositories" when the chosen one is no
    /// longer among the workspace's - a removed agent, or one whose folder
    /// changed, must not leave the filter pointed at a repository that can
    /// never show a row again.
    pub(super) fn reset_missing_repo(&mut self, available: &[RepoSlug]) {
        if self.repo
               .as_ref()
               .is_some_and(|repo| !available.contains(repo))
        {
            self.repo = None;
        }
    }
}

/// The search text as it is compared: trimmed and lowercased once, rather
/// than once per issue.
fn needle(search: &str) -> String {
    search.trim().to_lowercase()
}

/// Whether `issue` carries `needle` in its title, `#number`, repository as
/// `owner/repo`, URL, or any of its labels.
fn issue_matches(needle: &str, slug: &RepoSlug, issue: &Issue) -> bool {
    if needle.is_empty() {
        return true;
    }
    let haystacks = [issue.title.to_lowercase(),
                     format!("#{}", issue.number),
                     slug.to_string().to_lowercase(),
                     issue.url.to_lowercase()];
    haystacks.iter().any(|haystack| haystack.contains(needle))
    || issue.labels
            .iter()
            .any(|label| label.to_lowercase().contains(needle))
}

/// The groups the Issues tab draws: each workspace repository's issues that
/// pass the search and the repository filter, sorted within the group.
///
/// A repository with no issue left standing - either it has none, or the
/// search and filter hid all of it - is dropped rather than shown empty; the
/// tab tells the two apart by comparing against the unfiltered input.
/// Surviving groups come back ordered by `<owner>/<repo>`, per "The Issues
/// tab lists issues by repository".
// UNWIRED(#504): nothing calls this outside tests until `issues_pane.rs`
// asks it for the rows to draw.
#[allow(dead_code)]
pub(super) fn apply(groups: Vec<IssueGroup>, filter: &IssueFilter, sort: IssueSort)
                    -> Vec<IssueGroup> {
    let needle = needle(&filter.search);
    let mut groups: Vec<IssueGroup> =
        groups.into_iter()
              .filter(|group| filter.repo.as_ref().is_none_or(|repo| repo == &group.slug))
              .filter_map(|mut group| {
                  group.issues
                       .retain(|issue| issue_matches(&needle, &group.slug, issue));
                  if group.issues.is_empty() {
                      return None;
                  }
                  group.issues
                       .sort_by(|left, right| sort.compare(left, right));
                  Some(group)
              })
              .collect();
    groups.sort_by(|left, right| left.slug.cmp(&right.slug));
    groups
}

#[cfg(test)]
mod tests;
