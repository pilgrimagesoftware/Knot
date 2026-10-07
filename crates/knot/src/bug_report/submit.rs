//! Turning a report into a filed issue, or into a pre-filled compose page
//! when the forge cannot file it.
//!
//! Both paths target [`KNOT_REPO`] and carry the same [`issue_body`], so what
//! reaches GitHub does not depend on which one ran. Blocking: [`submit`] runs
//! `gh` or `open`, so it is only called off the main thread.
//!
//! Screenshots are named in the body and handed over by delivery rather
//! than uploaded - see [`super::screenshots`].

use std::path::{Path, PathBuf};

use knot_core::consts::KNOT_REPO;
use knot_forge::{ForgeAvailability, ForgeRunner};

use super::kind::IssueKind;
use super::logs::{Attachment, attachments_by_path, attachments_markdown};
use super::screenshots;

/// What the user filed, plus the diagnostics that accompany it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Report {
    pub(crate) kind:        IssueKind,
    pub(crate) subject:     String,
    pub(crate) description: String,
    pub(crate) diagnostics: String,
    /// The logs the user chose to attach, already read.
    pub(crate) attachments: Vec<Attachment>,
    /// The screenshots the user chose, by path. Named in the body, then
    /// shown in Finder for the user to drag onto the issue; never read.
    pub(crate) screenshots: Vec<PathBuf>,
}

/// How a submission ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Filed through `gh`, at this URL.
    Filed(String),
    /// `gh` was ready but filing failed, with why.
    FileFailed(String),
    /// The forge was not ready; the compose page opened in the browser.
    BrowserReady,
    /// The forge was not ready, and the browser could not be opened.
    BrowserFailed,
}

/// Files `report` through `runner` when `forge` is ready, otherwise hands the
/// compose page to `open_url`.
///
/// Anything short of ready - not installed, not authenticated, or a probe
/// that failed outright - falls back to the browser: the report still has a
/// way out, and the page is signed in as whoever the browser is.
///
/// A report with screenshots then puts them one drag from the issue: a filed
/// one opens the issue's page, and either path has `reveal` show each file
/// in Finder. A screenshot the reveal cannot show is still named in the body.
pub(crate) fn submit(report: &Report, forge: &ForgeAvailability, runner: &impl ForgeRunner,
                     open_url: impl Fn(&str) -> bool, reveal: impl Fn(&Path) -> bool)
                     -> Outcome {
    if forge.is_ready() {
        return match knot_forge::create_labeled_issue_with(runner,
                                                           KNOT_REPO,
                                                           report.subject.trim(),
                                                           &issue_body(report),
                                                           report.kind.github_label())
        {
            Ok(url) => {
                if !report.screenshots.is_empty() {
                    open_url(&url);
                    reveal_all(&report.screenshots, &reveal);
                }
                Outcome::Filed(url)
            }
            Err(error) => Outcome::FileFailed(error.to_string()),
        };
    }
    if open_url(&compose_url(report)) {
        reveal_all(&report.screenshots, &reveal);
        Outcome::BrowserReady
    }
    else {
        Outcome::BrowserFailed
    }
}

/// Shows each screenshot in Finder. A failure is not the report's: the issue
/// is filed or composed either way, and its body names every file.
fn reveal_all(screenshots: &[PathBuf], reveal: &impl Fn(&Path) -> bool) {
    for path in screenshots {
        reveal(path);
    }
}

/// The description, then the diagnostics under a heading, fenced so GitHub
/// keeps their lines as they are, then any attached logs.
pub(super) fn issue_body(report: &Report) -> String {
    body_with_logs(report, attachments_markdown(&report.attachments))
}

/// [`issue_body`] for the browser fallback, which names the log files
/// rather than carrying them: a compose URL that long is refused.
pub(super) fn compose_body(report: &Report) -> String {
    body_with_logs(report, attachments_by_path(&report.attachments))
}

fn body_with_logs(report: &Report, logs: String) -> String {
    let mut body = format!("{}\n\n### {}\n\n```\n{}\n```\n",
                           report.description.trim(),
                           knot_core::l10n::t("bug_report.diagnostics_label"),
                           report.diagnostics);
    if !report.attachments.is_empty() {
        body.push_str(&format!("\n### {}\n\n{logs}",
                               knot_core::l10n::t("bug_report.logs.label")));
    }
    body.push_str(&screenshots::body_section(&report.screenshots));
    body
}

/// The repository's new-issue page, with the title, body and label
/// pre-filled. GitHub applies the label only for a user allowed to set one,
/// and ignores it otherwise.
pub(super) fn compose_url(report: &Report) -> String {
    format!("https://github.com/{KNOT_REPO}/issues/new?title={}&body={}&labels={}",
            percent_encode(report.subject.trim()),
            percent_encode(&compose_body(report)),
            percent_encode(report.kind.github_label()))
}

/// Percent-encodes everything but RFC 3986's unreserved characters, so a
/// query value can carry spaces, slashes, `&`, `#` and any Unicode without
/// being cut short or misread.
pub(super) fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        }
        else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests;
