//! The Issues tab's state for a frame (#504): its search field, the groups it
//! draws, and which message - if any - stands in for them.
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! workspace-issues/spec.md`. [`issues_message`] is pure, so which of the tab's
//! states shows is tested without a window.

use std::collections::BTreeSet;

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{App, AppContext, Context, Entity, Window};
use knot_forge::{ForgeAvailability, RepoSlug};

use super::WorkspaceWindow;
use super::issue_filter::{self, IssueFilter, IssueGroup};
use super::work_items::IssueLookup;

/// What stands in for the list, when something does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum IssuesMessage {
    /// Nothing has answered yet: the probe, the repositories or the first
    /// fetch.
    Checking,
    ForgeMissing,
    ForgeUnauthenticated,
    ForgeFailed(String),
    /// No workspace repository is on GitHub.
    NoGithubRepos,
    /// Every repository answered, and none has an open issue.
    NoneOpen,
    /// There are issues, and the search or filter hides them all.
    NoneMatch,
}

/// What a frame of the tab draws.
pub(super) struct IssuesFrame {
    /// Every repository's groups as filtered and sorted for this frame.
    pub(super) shown:     Vec<IssueGroup>,
    /// Repositories whose fetch failed, with why, that the filter lets through.
    pub(super) failed:    Vec<(RepoSlug, String)>,
    /// Repositories with more open issues than were fetched.
    pub(super) truncated: BTreeSet<RepoSlug>,
    /// The repositories the picker offers, in order.
    pub(super) slugs:     Vec<RepoSlug>,
    pub(super) message:   Option<IssuesMessage>,
}

/// The facts [`issues_message`] decides from.
pub(super) struct IssuesFacts<'a> {
    pub(super) forge:          Option<&'a ForgeAvailability>,
    pub(super) repos_resolved: bool,
    pub(super) github_repos:   usize,
    /// Repositories that have answered, with issues or a failure.
    pub(super) answered:       usize,
    pub(super) total_issues:   usize,
    pub(super) shown_issues:   usize,
    pub(super) any_failed:     bool,
}

/// Which message the tab shows instead of rows, or `None` to draw them.
///
/// The `gh` states come first and say which fix applies - install it or sign
/// in. "No open issues" is claimed only when every repository answered and
/// none failed, and "nothing matches" only when there are issues to match.
pub(super) fn issues_message(facts: &IssuesFacts<'_>) -> Option<IssuesMessage> {
    match facts.forge {
        None => return Some(IssuesMessage::Checking),
        Some(ForgeAvailability::Missing) => return Some(IssuesMessage::ForgeMissing),
        Some(ForgeAvailability::Unauthenticated) => {
            return Some(IssuesMessage::ForgeUnauthenticated);
        }
        Some(ForgeAvailability::Failed(reason)) => {
            return Some(IssuesMessage::ForgeFailed(reason.clone()));
        }
        Some(ForgeAvailability::Ready) => {}
    }
    if !facts.repos_resolved {
        return Some(IssuesMessage::Checking);
    }
    if facts.github_repos == 0 {
        return Some(IssuesMessage::NoGithubRepos);
    }
    if facts.answered == 0 {
        return Some(IssuesMessage::Checking);
    }
    if facts.total_issues == 0 {
        return (!facts.any_failed && facts.answered == facts.github_repos)
            .then_some(IssuesMessage::NoneOpen);
    }
    (facts.shown_issues == 0 && !facts.any_failed).then_some(IssuesMessage::NoneMatch)
}

impl WorkspaceWindow {
    /// The search field, created on the first frame that needs it.
    pub(super) fn issues_search(&mut self, window: &mut Window, cx: &mut Context<Self>)
                                -> Entity<InputState> {
        if let Some(input) = &self.changes_view.issues.search {
            return input.clone();
        }
        let input = cx.new(|cx| {
                          InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("issues.search_placeholder"))
                .clean_on_escape()
                      });
        let subscription = cx.subscribe_in(&input, window, |_: &mut Self, _, event, _, cx| {
                                 if matches!(event, InputEvent::Change) {
                                     cx.notify();
                                 }
                             });
        self.changes_view.issues.search = Some(input.clone());
        self.changes_view.issues._search_changes = Some(subscription);
        input
    }

    /// The tab's frame: groups, failures, truncation and the message.
    ///
    /// A repository filter naming a repository the workspace no longer has
    /// goes back to all repositories here, rather than leaving the tab empty
    /// for a reason the user can no longer see.
    pub(super) fn issues_frame(&mut self, cx: &App) -> IssuesFrame {
        let repos = self.work_items.repos.get(&());
        let slugs: Vec<RepoSlug> = repos.iter()
                                        .flatten()
                                        .filter_map(|repo| repo.slug.clone())
                                        .collect();
        let lookups = self.work_items.issues.snapshot();
        let mut filter = IssueFilter { search: self.changes_view
                                                   .issues
                                                   .search
                                                   .as_ref()
                                                   .map(|input| input.read(cx).value().to_string())
                                                   .unwrap_or_default(),
                                       repo:   self.changes_view.issues.repo.clone(), };
        filter.reset_missing_repo(&slugs);
        self.changes_view.issues.repo = filter.repo.clone();

        let mut groups = Vec::new();
        let mut failed = Vec::new();
        let mut truncated = BTreeSet::new();
        for slug in &slugs {
            match lookups.get(slug) {
                Some(IssueLookup::Page(page)) => {
                    if page.truncated {
                        truncated.insert(slug.clone());
                    }
                    groups.push(IssueGroup { slug:   slug.clone(),
                                             issues: page.issues.clone(), });
                }
                Some(IssueLookup::Failed(reason))
                    if filter.repo.as_ref().is_none_or(|chosen| chosen == slug) =>
                {
                    failed.push((slug.clone(), reason.clone()));
                }
                Some(IssueLookup::Failed(_)) | None => {}
            }
        }
        let total_issues = groups.iter().map(|group| group.issues.len()).sum();
        let shown = issue_filter::apply(groups, &filter, self.changes_view.issues.sort);
        let forge = self.forge_status.availability();
        let facts =
            IssuesFacts { forge: forge.as_ref(),
                          repos_resolved: repos.is_some(),
                          github_repos: slugs.len(),
                          answered: slugs.iter()
                                         .filter(|slug| lookups.contains_key(*slug))
                                         .count(),
                          total_issues,
                          shown_issues: shown.iter().map(|group| group.issues.len()).sum(),
                          any_failed: !failed.is_empty() };
        IssuesFrame { message: issues_message(&facts),
                      shown,
                      failed,
                      truncated,
                      slugs }
    }

    /// Empty the search and return to all repositories. The sort stays: it
    /// is a preference, not a narrowing.
    pub(super) fn clear_issue_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.changes_view.issues.repo = None;
        if let Some(search) = &self.changes_view.issues.search {
            search.update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests;
