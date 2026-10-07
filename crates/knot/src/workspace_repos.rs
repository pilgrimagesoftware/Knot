//! The git repositories a workspace's agents work in (#504), for the Changes
//! view's Issues and OpenSpec tabs.
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! workspace-issues/spec.md`, "A workspace's repositories come from its agents'
//! folders". A workspace names agents, not repositories, so the repositories
//! are read off the agents' folders: worktrees of one repository are one
//! repository (keyed by git's common dir), a subfolder belongs to its
//! repository, a folder outside git contributes nothing, and only a GitHub
//! `origin` gets a slug - the Issues tab needs one, the OpenSpec tab does not.
//!
//! [`resolve`] runs `git`, so it is only called off the main thread;
//! [`group`] is the pure part and is tested without a repository.

use std::path::{Path, PathBuf};

use knot_forge::RepoSlug;

/// One repository the workspace's agents work in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceRepo {
    /// Git's common dir: the same for every worktree of the repository.
    pub(crate) common_dir: PathBuf,
    /// The working trees agents work in, in the order first seen, each once.
    pub(crate) worktrees:  Vec<PathBuf>,
    /// `owner/repo` when `origin` is on GitHub.
    pub(crate) slug:       Option<RepoSlug>,
    /// The group heading: the slug, else the repository's folder name.
    pub(crate) label:      String,
}

/// What `git` says about one agent folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedFolder {
    pub(crate) toplevel:   PathBuf,
    pub(crate) common_dir: PathBuf,
    pub(crate) slug:       Option<RepoSlug>,
}

/// Reads `folders` with `git`. Blocks: off the main thread only.
pub(crate) fn resolve(folders: &[PathBuf]) -> Vec<WorkspaceRepo> {
    group(folders.iter().filter_map(|folder| resolve_folder(folder)))
}

/// One folder, or `None` outside a repository. A failed `git` reads as
/// "not a repository" - the tab shows what it can rather than an error for
/// a folder the user may simply not have cloned yet.
fn resolve_folder(folder: &Path) -> Option<ResolvedFolder> {
    let repository = knot_git::Repository::open(folder);
    let toplevel = repository.toplevel().ok().flatten()?;
    let common_dir = repository.common_dir().ok().flatten()?;
    let slug = repository.remote_url("origin")
                         .ok()
                         .flatten()
                         .and_then(|url| knot_forge::github_repo(&url));
    Some(ResolvedFolder { toplevel,
                          common_dir,
                          slug })
}

/// Merges resolved folders into repositories by common dir, ordered by
/// label. Two agents in one working tree, or in subfolders of it, list that
/// tree once.
pub(crate) fn group(resolved: impl IntoIterator<Item = ResolvedFolder>) -> Vec<WorkspaceRepo> {
    let mut repos: Vec<WorkspaceRepo> = Vec::new();
    for folder in resolved {
        match repos.iter_mut()
                   .find(|repo| repo.common_dir == folder.common_dir)
        {
            Some(repo) => {
                if !repo.worktrees.contains(&folder.toplevel) {
                    repo.worktrees.push(folder.toplevel);
                }
                if repo.slug.is_none() {
                    repo.slug = folder.slug;
                }
            }
            None => repos.push(WorkspaceRepo { label:      String::new(),
                                               common_dir: folder.common_dir,
                                               worktrees:  vec![folder.toplevel],
                                               slug:       folder.slug, }),
        }
    }
    for repo in &mut repos {
        repo.label = label(repo);
    }
    repos.sort_by(|a, b| a.label.cmp(&b.label));
    repos
}

/// `owner/repo` on GitHub. Otherwise the repository's own folder name: for
/// a common dir `<repo>/.git`, the name of `<repo>`, so every worktree of it
/// is headed the same whatever its own folder is called; else the first
/// working tree's folder name.
fn label(repo: &WorkspaceRepo) -> String {
    if let Some(slug) = &repo.slug {
        return slug.to_string();
    }
    let from_common_dir = (repo.common_dir.file_name().and_then(|name| name.to_str())
                           == Some(".git")).then(|| repo.common_dir.parent())
                                           .flatten()
                                           .and_then(Path::file_name);
    from_common_dir.or_else(|| repo.worktrees.first().and_then(|tree| tree.file_name()))
                   .map_or_else(|| repo.common_dir.display().to_string(),
                                |name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests;
