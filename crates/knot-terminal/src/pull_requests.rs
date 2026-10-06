//! Watching a terminal agent's output for pull request URLs.
//!
//! Contract: the `pull-request-tracking` capability spec under
//! `openspec/changes/pull-request-tracking/specs/`.
//!
//! The scan is on the PTY byte stream, through the terminal's output hook
//! (`gpui_terminal::TerminalBuilder::on_output`), not on the rendered grid.
//! Scanning the grid would mean re-reading a screen that changes every frame,
//! on a path that runs per repaint, and would miss anything that scrolled past
//! between two polls. The stream sees every byte exactly once.
//!
//! The scanner carries a bounded tail across chunk boundaries, because a PTY
//! read can split a URL anywhere.

use knot_core::pull_request_url::PullRequestUrlScanner;

/// The scanner and what it has found, fed from the output hook on the PTY
/// reader thread and drained by the window's poll.
#[derive(Default)]
pub(crate) struct PullRequestTap {
    /// Carries a bounded tail across chunk boundaries, since a PTY read can
    /// split a URL anywhere.
    scanner: PullRequestUrlScanner,
    /// Found but not yet handed to the window that owns the agent store.
    urls:    Vec<String>,
}

impl PullRequestTap {
    /// Scan a chunk of PTY output, buffering any pull request URLs found.
    pub(crate) fn feed(&mut self, bytes: &[u8]) {
        for url in self.scanner.feed(bytes, PULL_REQUEST_HOSTS) {
            let url = url.to_string();
            if !self.urls.contains(&url) {
                self.urls.push(url);
            }
        }
    }

    /// Take the pull request URLs seen since the last call.
    ///
    /// Drained rather than read, so the window records each sighting once
    /// however often it polls, and the buffer does not grow for the life of
    /// the session. Recording is idempotent per agent anyway, so a URL handed
    /// over twice costs nothing.
    pub(crate) fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut self.urls)
    }
}

/// The hosts a terminal agent's output is scanned against.
///
/// `github.com` only. An Enterprise host would have to come from `gh`, which
/// this crate has no business asking - and a URL Knot cannot look up is not
/// one it records.
const PULL_REQUEST_HOSTS: &[&str] = &[knot_core::consts::PULL_REQUEST_HOST_GITHUB];

#[cfg(test)]
mod tests;
