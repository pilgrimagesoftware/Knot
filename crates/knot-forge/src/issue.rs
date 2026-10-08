//! Filing an issue, and listing a repository's open ones.
//!
//! Filing is the bug-report dialog's one write to the forge. Listing is the
//! Issues tab's one read, through `gh issue list --repo`, which already
//! excludes pull requests. Both go through the same runner as every other
//! read, so they inherit the located binary, the timeout and the error
//! mapping, and never need a credential of their own.
//!
//! Contract: "A repository's open issues are fetched and refreshed" in
//! `openspec/changes/pr-view-issues-and-changes/specs/workspace-issues/spec.
//! md`.

use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::consts::{ISSUE_LIST_FIELDS, ISSUE_LIST_LIMIT};
use crate::error::{ForgeError, Result};
use crate::repo_slug::RepoSlug;
use crate::runner::{ForgeRunner, GhRunner};

/// File an issue on `repo` (`owner/name`) with the real `gh`, returning the
/// new issue's URL.
pub fn create_issue(repo: &str, title: &str, body: &str) -> Result<String> {
    create_issue_with(&GhRunner::new(), repo, title, body)
}

/// [`create_issue`], against a supplied runner.
///
/// Title and body are separate argv elements, never a shell line, so nothing
/// the user typed can be read as an option or a command. `gh issue create`
/// prints the new issue's URL on its last line; anything before it is
/// progress chatter.
pub fn create_issue_with(runner: &impl ForgeRunner, repo: &str, title: &str, body: &str)
                         -> Result<String> {
    let stdout =
        runner.run(&["issue", "create", "--repo", repo, "--title", title, "--body", body])?;

    issue_url(&stdout)
}

/// [`create_issue_with`], filed under `label` where the forge allows it.
///
/// GitHub lets only users with triage access label an issue, and a reporter
/// is usually not one, so a rejected filing is tried once more without the
/// label: an unlabeled report beats none. Only a command failure is retried -
/// a timeout may have filed the issue already, and a missing binary would
/// fail the same way twice.
pub fn create_labeled_issue_with(runner: &impl ForgeRunner, repo: &str, title: &str, body: &str,
                                 label: &str)
                                 -> Result<String> {
    let stdout = runner.run(&["issue", "create", "--repo", repo, "--title", title, "--body",
                              body, "--label", label]);
    match stdout {
        Ok(stdout) => issue_url(&stdout),
        Err(ForgeError::Command { .. }) => create_issue_with(runner, repo, title, body),
        Err(error) => Err(error),
    }
}

/// `gh issue create` prints the new issue's URL on its last line.
fn issue_url(stdout: &str) -> Result<String> {
    stdout.lines()
          .map(str::trim)
          .rfind(|line| line.starts_with("https://"))
          .map(str::to_owned)
          .ok_or_else(|| ForgeError::Parse(format!("no issue URL in gh output: {stdout}")))
}

/// One open issue, as the Issues tab shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub number:     u64,
    pub title:      String,
    pub url:        String,
    pub labels:     Vec<String>,
    /// `None` when the issue has no author - `gh` reports this for an
    /// author whose account no longer exists, not for a missing field.
    pub author:     Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// A repository's open issues, capped at the limit asked for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IssuePage {
    /// At most the limit [`issue_list_with`] was asked for.
    pub issues:    Vec<Issue>,
    /// Whether the repository has more open issues than [`Self::issues`]
    /// holds.
    pub truncated: bool,
}

/// List `slug`'s open issues with the real `gh`, most recently updated
/// first, capped at [`ISSUE_LIST_LIMIT`].
pub fn issue_list(slug: &RepoSlug) -> Result<IssuePage> {
    issue_list_with(&GhRunner::new(), slug, ISSUE_LIST_LIMIT)
}

/// [`issue_list`], against a supplied runner and an explicit `limit`.
///
/// Asks for one more than `limit` so a repository with exactly `limit` open
/// issues is not reported as truncated: the extra row, if it comes back, is
/// evidence there is more rather than part of the answer, and is dropped
/// before this returns. `gh issue list` already excludes pull requests, so
/// no further filtering is needed here.
pub fn issue_list_with(runner: &impl ForgeRunner, slug: &RepoSlug, limit: usize)
                       -> Result<IssuePage> {
    let repo = slug.to_string();
    let asked = (limit + 1).to_string();
    let stdout = runner.run(&["issue",
                              "list",
                              "--repo",
                              &repo,
                              "--state",
                              "open",
                              "--limit",
                              &asked,
                              "--json",
                              ISSUE_LIST_FIELDS])?;
    parse_issue_page(&stdout, limit)
}

/// What `gh issue list --json` sends back for one issue.
///
/// `deny_unknown_fields` is deliberately absent, as in `RawPullRequest`: a
/// field a newer `gh` volunteers is not a reason to lose the row.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIssue {
    number:     u64,
    title:      String,
    url:        String,
    #[serde(default)]
    labels:     Vec<RawLabel>,
    #[serde(default)]
    author:     Option<RawAuthor>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
struct RawLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawAuthor {
    login: String,
}

/// Turn one `gh issue list --json` payload into a page, dropping whatever
/// came back past `limit` and reporting whether anything was dropped.
fn parse_issue_page(json: &str, limit: usize) -> Result<IssuePage> {
    let raw: Vec<RawIssue> =
        serde_json::from_str(json).map_err(|err| ForgeError::Parse(err.to_string()))?;

    let truncated = raw.len() > limit;
    let issues = raw.into_iter()
                    .take(limit)
                    .map(issue_from_raw)
                    .collect::<Result<Vec<_>>>()?;

    Ok(IssuePage { issues, truncated })
}

fn issue_from_raw(raw: RawIssue) -> Result<Issue> {
    Ok(Issue { number:     raw.number,
               title:      raw.title,
               url:        raw.url,
               labels:     raw.labels.into_iter().map(|label| label.name).collect(),
               author:     raw.author.map(|author| author.login),
               created_at: parse_timestamp(&raw.created_at)?,
               updated_at: parse_timestamp(&raw.updated_at)?, })
}

fn parse_timestamp(raw: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(raw, &Rfc3339).map_err(|err| ForgeError::Parse(err.to_string()))
}

#[cfg(test)]
mod tests;
