//! Identifying a GitHub repository from the URL of a git remote.
//!
//! Contract: "A workspace's repositories come from its agents' folders" in
//! `openspec/changes/pr-view-issues-and-changes/specs/workspace-issues/spec.
//! md`.
//!
//! Pure string parsing, no process and no network - `workspace_repos`
//! resolves a remote's URL through `knot-git`, then asks here whether it
//! names a GitHub repository.

use std::fmt;

/// `owner/repo` on GitHub, identified from a remote URL.
///
/// Ordered by `(owner, repo)`, which is the same order as [`Display`] - what
/// the Issues tab's grouping sorts repository headings by.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoSlug {
    pub owner: String,
    pub repo:  String,
}

impl fmt::Display for RepoSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

/// The GitHub `owner/repo` a remote URL names, or `None` when it is
/// malformed or names a different host.
///
/// Accepts the four forms `git remote get-url` sends back for a GitHub
/// remote, with or without a trailing `.git`:
/// - `https://github.com/owner/repo`
/// - `git@github.com:owner/repo`
/// - `ssh://git@github.com/owner/repo`
///
/// A remote on any other host, or a GitHub URL with anything other than
/// exactly an owner and a repository in its path (a blob URL, an org page,
/// an empty segment), is `None` rather than a best-effort guess: a wrong
/// guess would fetch another repository's issues under this one's heading.
pub fn github_repo(remote_url: &str) -> Option<RepoSlug> {
    let (host, path) = split_host_and_path(remote_url.trim())?;
    if !host.eq_ignore_ascii_case("github.com") {
        return None;
    }
    parse_owner_repo(path)
}

/// Splits a remote URL into its host and path, across the `ssh://`, `git@`
/// and `https://` forms. The path is handed back exactly as it followed the
/// host - still possibly carrying a leading `/` or a trailing `.git`, which
/// [`parse_owner_repo`] strips.
fn split_host_and_path(url: &str) -> Option<(&str, &str)> {
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        return rest.split_once('/');
    }
    if let Some(rest) = url.strip_prefix("https://") {
        return rest.split_once('/');
    }
    if let Some(rest) = url.strip_prefix("git@") {
        return rest.split_once(':');
    }
    None
}

/// `owner/repo` from a path, after trimming the slashes and `.git` suffix
/// git's own forms carry. `None` unless exactly two non-empty segments
/// remain.
fn parse_owner_repo(path: &str) -> Option<RepoSlug> {
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);

    let mut segments = path.split('/');
    let owner = segments.next().filter(|s| !s.is_empty())?;
    let repo = segments.next().filter(|s| !s.is_empty())?;
    if segments.next().is_some() {
        return None;
    }

    Some(RepoSlug { owner: owner.to_owned(),
                    repo:  repo.to_owned(), })
}

#[cfg(test)]
mod tests;
