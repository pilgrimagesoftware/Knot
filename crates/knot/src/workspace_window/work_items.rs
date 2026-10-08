//! The Changes view's Issues and OpenSpec tabs' data (#504): which
//! repositories the workspace works in, their open issues, and their
//! un-archived OpenSpec changes.
//!
//! Contracts: `workspace-issues` and `workspace-openspec-changes` under
//! `openspec/changes/pr-view-issues-and-changes/specs/`.
//!
//! Three claim-refresh caches, the Pull Requests tab's pattern: a render
//! asks each for what it has and claims a refresh of what has aged out, the
//! work runs in `spawn_blocking`, and `repaint_poll_tick` reads each cache's
//! changed flag. Nothing is fetched for a tab that is not shown. Issues wait
//! for the repositories, since a slug is the key they are fetched by.

use std::path::PathBuf;

use gpui_kit::App;
use knot_forge::{GhRunner, IssuePage, RepoSlug};

use super::WorkspaceWindow;
use super::changes_tab::ChangesTab;
use crate::consts::{ISSUES_MAX_AGE, OPENSPEC_CHANGES_MAX_AGE, WORKSPACE_REPOS_MAX_AGE};
use crate::openspec_changes::{self, ChangeEntry};
use crate::refresh_cache::RefreshCache;
use crate::workspace_repos::{self, WorkspaceRepo};

/// One repository's issues as the Issues tab last heard them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IssueLookup {
    Page(IssuePage),
    /// The fetch failed, with why - shown in that repository's group while
    /// the others list normally.
    Failed(String),
}

/// The three caches, one per window.
#[derive(Default)]
pub(crate) struct WorkItemCaches {
    /// The workspace's repositories, under a single key.
    pub(crate) repos:   RefreshCache<(), Vec<WorkspaceRepo>>,
    /// Open issues per GitHub repository.
    pub(crate) issues:  RefreshCache<RepoSlug, IssueLookup>,
    /// Un-archived changes per repository, by common dir.
    pub(crate) changes: RefreshCache<PathBuf, Vec<ChangeEntry>>,
}

impl WorkItemCaches {
    /// Whether anything landed since the last tick, clearing every flag.
    /// All three are read into locals before they are combined: `||` would
    /// skip a later consuming read and strand its flag (see
    /// `.claude/rules/rust-structure.md`).
    pub(crate) fn take_changed(&self) -> bool {
        let repos = self.repos.take_changed();
        let issues = self.issues.take_changed();
        let changes = self.changes.take_changed();
        repos || issues || changes
    }
}

impl WorkspaceWindow {
    /// The repositories as last resolved, or none yet.
    pub(super) fn workspace_repos(&self) -> Vec<WorkspaceRepo> {
        self.work_items.repos.get(&()).unwrap_or_default()
    }

    /// Refresh what has aged out for the tab being shown. Called every
    /// frame; nothing here blocks - each claim hands its writer straight to
    /// `spawn_blocking`.
    pub(super) fn refresh_work_items(&mut self, cx: &App) {
        let issues = self.showing_changes_tab(ChangesTab::Issues);
        let openspec = self.showing_changes_tab(ChangesTab::OpenSpec);
        if !issues && !openspec {
            return;
        }
        if let Some(writer) = self.work_items
                                  .repos
                                  .claim_refresh((), WORKSPACE_REPOS_MAX_AGE)
        {
            let folders = self.agent_folders();
            self.runtime
                .spawn_blocking(move || writer.record(workspace_repos::resolve(&folders)));
        }
        let repos = self.workspace_repos();
        if issues {
            self.refresh_issues(&repos, cx);
        }
        if openspec {
            self.refresh_openspec_changes(&repos);
        }
    }

    fn refresh_issues(&mut self, repos: &[WorkspaceRepo], _cx: &App) {
        // The probe the Pull Requests tab already keeps: one `gh auth status`
        // for every tab, and none of the per-repository calls while it says
        // `gh` is missing or signed out.
        if let Some(writer) = self.forge_status
                                  .claim_probe(crate::pull_request_state::PROBE_MAX_AGE)
        {
            self.runtime
                .spawn_blocking(move || writer.record(knot_forge::probe()));
        }
        if !self.forge_status.is_ready() {
            return;
        }
        let slugs: Vec<RepoSlug> = repos.iter().filter_map(|repo| repo.slug.clone()).collect();
        // A repository no agent works in any more takes its issues with it.
        self.work_items.issues.retain(|slug| slugs.contains(slug));
        for slug in slugs {
            let Some(writer) = self.work_items
                                   .issues
                                   .claim_refresh(slug.clone(), ISSUES_MAX_AGE)
            else {
                continue;
            };
            self.runtime.spawn_blocking(move || {
                            let lookup = match knot_forge::issue_list_with(&GhRunner::new(),
                                                                         &slug,
                                                                         knot_forge::consts::ISSUE_LIST_LIMIT)
                            {
                                Ok(page) => IssueLookup::Page(page),
                                Err(error) => IssueLookup::Failed(error.to_string()),
                            };
                            writer.record(lookup);
                        });
        }
    }

    fn refresh_openspec_changes(&mut self, repos: &[WorkspaceRepo]) {
        let dirs: Vec<PathBuf> = repos.iter().map(|repo| repo.common_dir.clone()).collect();
        self.work_items.changes.retain(|dir| dirs.contains(dir));
        for repo in repos {
            let Some(writer) =
                self.work_items
                    .changes
                    .claim_refresh(repo.common_dir.clone(), OPENSPEC_CHANGES_MAX_AGE)
            else {
                continue;
            };
            let worktrees = repo.worktrees.clone();
            self.runtime.spawn_blocking(move || {
                            writer.record(openspec_changes::merge(worktrees.iter()
                                                                           .map(|tree| {
                                                                               openspec_changes::scan(tree)
                                                                           })));
                        });
        }
    }

    /// Refresh now on the Issues tab: re-probe `gh`, re-read the
    /// repositories and refetch every repository's issues, keeping what is
    /// shown until the answers land.
    pub(super) fn refresh_issues_now(&mut self) {
        // Re-probe too, per the spec: Refresh now is how a user who just
        // installed or signed in to `gh` gets the tab to notice.
        self.forge_status.mark_stale();
        self.work_items.repos.mark_all_stale();
        self.work_items.issues.mark_all_stale();
    }

    /// Refresh now on the OpenSpec tab.
    pub(super) fn refresh_openspec_now(&mut self) {
        self.work_items.repos.mark_all_stale();
        self.work_items.changes.mark_all_stale();
    }

    /// Each distinct agent folder in the workspace.
    fn agent_folders(&self) -> Vec<PathBuf> {
        let ids = self.workspace_agent_ids();
        let store = self.store.lock();
        let mut folders: Vec<PathBuf> = Vec::new();
        for id in ids {
            if let Some(agent) = store.agent(id) {
                let folder = PathBuf::from(&agent.folder);
                if !folders.contains(&folder) {
                    folders.push(folder);
                }
            }
        }
        folders
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// A claimed refresh of an aged-out key goes out once, and the answer
    /// landing is what the repaint tick sees - once, for any of the three
    /// caches.
    #[test]
    fn a_landed_answer_reports_the_caches_changed_once() {
        let mut caches = WorkItemCaches::default();
        assert!(!caches.take_changed());

        let writer = caches.changes
                           .claim_refresh(PathBuf::from("/src/widget/.git"), Duration::ZERO)
                           .expect("a key never asked about is stale");
        assert!(caches.changes
                      .claim_refresh(PathBuf::from("/src/widget/.git"), Duration::from_secs(60))
                      .is_none(),
                "a fresh claim is not handed out twice");
        writer.record(Vec::new());

        assert!(caches.take_changed(), "the tick sees the scan land");
        assert!(!caches.take_changed(), "and only once");
    }

    /// Each cache's flag is read even when an earlier one was set, so none
    /// is stranded to fire a spurious repaint on the next tick.
    #[test]
    fn every_flag_is_cleared_in_one_tick() {
        let mut caches = WorkItemCaches::default();
        caches.repos
              .claim_refresh((), Duration::ZERO)
              .unwrap()
              .record(Vec::new());
        caches.changes
              .claim_refresh(PathBuf::from("/a"), Duration::ZERO)
              .unwrap()
              .record(Vec::new());

        assert!(caches.take_changed());
        assert!(!caches.take_changed(), "a flag survived the first tick");
    }
}
