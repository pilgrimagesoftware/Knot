//! Filing an issue.
//!
//! The bug-report dialog's one write to the forge. Through the same runner as
//! every read, so it inherits the located binary, the timeout and the error
//! mapping, and never needs a credential of its own.

use crate::error::{ForgeError, Result};
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

    stdout.lines()
          .map(str::trim)
          .rfind(|line| line.starts_with("https://"))
          .map(str::to_owned)
          .ok_or_else(|| ForgeError::Parse(format!("no issue URL in gh output: {stdout}")))
}

#[cfg(test)]
mod tests;
