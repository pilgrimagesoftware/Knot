//! Recognizing a pull request URL in an agent's output.
//!
//! Implements the detection half of
//! `openspec/changes/pull-request-tracking/specs/pull-request-tracking/spec.
//! md`.
//!
//! Knot detects a pull request by its URL rather than by the command that
//! printed it. The ACP stream has no typed command field, terminal agents
//! have no stream at all, and the ways to open a pull request are
//! open-ended - `gh`, the hint `git push` prints, an MCP tool, a paste from
//! the browser. All of them put the URL in the output; nothing else is
//! common to all of them.
//!
//! Parsing into [`PullRequestUrl`] rather than keeping the matched text is
//! what makes `/pull/42` and `/pull/42/files` the same record: the suffix is
//! never captured, so the two spellings cannot disagree.
//!
//! Two entry points, because the two agent kinds carry output differently:
//! [`scan_pull_request_urls`] for text that arrives whole (a panel agent's
//! tool-call content), and [`PullRequestUrlScanner`] for bytes that arrive in
//! arbitrary chunks (a terminal agent's PTY stream).

use core::fmt;

use crate::consts::{MAX_PULL_REQUEST_URL_LEN, PULL_REQUEST_HOST_GITHUB, PULL_REQUEST_URL_SCHEME};

/// A pull request Knot has seen, parsed into its parts.
///
/// Held apart rather than as the matched string so that two spellings of one
/// pull request compare equal. [`fmt::Display`] writes the canonical form,
/// which is what is recorded and what is opened in a browser.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PullRequestUrl {
    /// `github.com`, or the GitHub Enterprise host the URL was found on.
    pub host:   String,
    /// The owner segment: a user or organization.
    pub owner:  String,
    /// The repository segment.
    pub repo:   String,
    /// The pull request number. Always positive; `/pull/0` does not parse.
    pub number: u64,
}

impl fmt::Display for PullRequestUrl {
    /// The canonical form: scheme, host, owner, repo, number, and nothing
    /// else. Any `/files`, `?query` or `#fragment` the output carried is
    /// absent, because it was never parsed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
               "{PULL_REQUEST_URL_SCHEME}{}/{}/{}/pull/{}",
               self.host, self.owner, self.repo, self.number)
    }
}

/// Find every pull request URL in `text`, on the hosts Knot recognizes by
/// default.
///
/// Duplicates are returned as many times as they occur; collapsing them is
/// the store's job, since idempotence is per agent rather than per chunk.
pub fn scan_pull_request_urls(text: &str) -> Vec<PullRequestUrl> {
    scan_pull_request_urls_on_hosts(text, &[PULL_REQUEST_HOST_GITHUB])
}

/// [`scan_pull_request_urls`], against an explicit host list.
///
/// A URL on a host that is not listed is not a pull request Knot can look up,
/// so it is not one Knot records. The forge layer supplies the Enterprise
/// hosts it knows are configured; nothing here guesses at them.
pub fn scan_pull_request_urls_on_hosts(text: &str, hosts: &[&str]) -> Vec<PullRequestUrl> {
    let mut found = Vec::new();
    scan_bytes(text.as_bytes(), hosts, true, &mut found, &mut 0);
    found
}

/// Scans a byte stream that arrives in arbitrary chunks.
///
/// A PTY read can split a URL anywhere, so the scanner carries the tail of
/// each chunk into the next one. It carries at most
/// [`MAX_PULL_REQUEST_URL_LEN`] bytes: a URL longer than that cannot parse,
/// so nothing further back can still become one.
///
/// Bytes, not text, because a PTY carries whatever the program wrote -
/// including invalid UTF-8 - and a pull request URL is ASCII either way.
#[derive(Debug, Default)]
pub struct PullRequestUrlScanner {
    carry: Vec<u8>,
}

impl PullRequestUrlScanner {
    /// A scanner with nothing carried over.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed the next chunk, returning the pull request URLs it completed.
    ///
    /// A URL whose digits run to the end of what has been fed is held back
    /// rather than returned: the next chunk may continue it, and `/pull/4`
    /// followed by `2` is pull request 42, not pull request 4. [`Self::flush`]
    /// resolves the held-back case when there is no next chunk.
    pub fn feed(&mut self, chunk: &[u8], hosts: &[&str]) -> Vec<PullRequestUrl> {
        self.scan(chunk, hosts, false)
    }

    /// Feed the last chunk, returning everything left - including a URL that
    /// ends exactly at the end of the stream.
    pub fn flush(&mut self, hosts: &[&str]) -> Vec<PullRequestUrl> {
        let found = self.scan(&[], hosts, true);
        self.carry.clear();
        found
    }

    fn scan(&mut self, chunk: &[u8], hosts: &[&str], at_end: bool) -> Vec<PullRequestUrl> {
        self.carry.extend_from_slice(chunk);
        let mut found = Vec::new();
        let mut consumed = 0;
        scan_bytes(&self.carry, hosts, at_end, &mut found, &mut consumed);
        // Everything up to the last match is settled; re-scanning it would
        // report the same URL twice. Of what is left, only the last
        // MAX_PULL_REQUEST_URL_LEN bytes can still begin a URL.
        self.carry.drain(..consumed);
        if self.carry.len() > MAX_PULL_REQUEST_URL_LEN {
            self.carry
                .drain(..self.carry.len() - MAX_PULL_REQUEST_URL_LEN);
        }
        found
    }
}

#[cfg(test)]
thread_local! {
    /// Bytes handed to [`scan_bytes`] on this thread since it was last reset.
    ///
    /// The streaming scan runs on the same thread that parses PTY bytes into
    /// cells, so its cost has to stay linear in the stream. That is a claim
    /// about work done, and this is what lets a test assert it as one rather
    /// than timing the scan and hoping the machine holds still - see
    /// `tests::the_streaming_scan_does_work_linear_in_the_stream`.
    ///
    /// Thread-local rather than a global counter because the test suite runs
    /// tests in parallel and a shared counter would make the assertion depend
    /// on what else happened to be running - reintroducing exactly the
    /// non-determinism it exists to remove.
    pub(super) static BYTES_SCANNED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

/// The shared scan. `at_end` says whether the input is the whole of what
/// there will ever be, which decides a match whose digits touch the end.
/// `consumed` reports the offset just past the last match, so a streaming
/// caller knows what it can drop.
fn scan_bytes(input: &[u8], hosts: &[&str], at_end: bool, found: &mut Vec<PullRequestUrl>,
              consumed: &mut usize) {
    #[cfg(test)]
    BYTES_SCANNED.with(|scanned| scanned.set(scanned.get() + input.len() as u64));

    let scheme = PULL_REQUEST_URL_SCHEME.as_bytes();
    let mut at = 0;
    while at + scheme.len() <= input.len() {
        let Some(offset) = find(&input[at..], scheme)
        else {
            break;
        };
        let start = at + offset;
        if preceded_by_url_char(input, start) {
            at = start + scheme.len();
            continue;
        }
        match parse_at(&input[start..], hosts, at_end) {
            Some((url, len)) => {
                found.push(url);
                at = start + len;
                *consumed = at;
            }
            None => at = start + scheme.len(),
        }
    }
}

/// Parse a pull request URL starting at the beginning of `input`, returning
/// it and the byte length it occupied. `None` for anything else, including a
/// URL whose number is still arriving.
fn parse_at(input: &[u8], hosts: &[&str], at_end: bool) -> Option<(PullRequestUrl, usize)> {
    let mut rest = input.strip_prefix(PULL_REQUEST_URL_SCHEME.as_bytes())?;
    let mut len = PULL_REQUEST_URL_SCHEME.len();

    let (host, taken) = take_while(rest, is_host_char)?;
    if !hosts.iter().any(|known| known.eq_ignore_ascii_case(host)) {
        return None;
    }
    rest = &rest[taken..];
    len += taken;

    let taken = take_byte(rest, b'/')?;
    rest = &rest[taken..];
    len += taken;

    let (owner, taken) = take_while(rest, is_owner_char)?;
    rest = &rest[taken..];
    len += taken;

    let taken = take_byte(rest, b'/')?;
    rest = &rest[taken..];
    len += taken;

    let (repo, taken) = take_while(rest, is_repo_char)?;
    rest = &rest[taken..];
    len += taken;

    let segment = b"/pull/";
    if !rest.starts_with(segment) {
        return None;
    }
    rest = &rest[segment.len()..];
    len += segment.len();

    let (digits, taken) = take_while(rest, |byte| byte.is_ascii_digit())?;
    // Digits running to the end of the input are only a whole number if the
    // input is the whole stream. Mid-stream they may still be growing.
    if taken == rest.len() && !at_end {
        return None;
    }
    // A path segment ends; it does not run into more of the same segment.
    // A full stop does end it, because prose puts one after a URL far more
    // often than GitHub puts one in a number.
    if let Some(&next) = rest.get(taken)
       && is_number_tail_char(next)
    {
        return None;
    }
    let number: u64 = digits.parse().ok()?;
    if number == 0 {
        return None;
    }
    len += taken;

    Some((PullRequestUrl { host: host.to_owned(),
                           owner: owner.to_owned(),
                           repo: repo.to_owned(),
                           number },
          len))
}

/// Whether the byte before `start` would make this `https://` part of a
/// longer token rather than the start of a URL.
fn preceded_by_url_char(input: &[u8], start: usize) -> bool {
    start > 0 && is_repo_char(input[start - 1])
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len())
            .position(|window| window == needle)
}

/// The longest run of bytes matching `predicate`, as text, with its length.
/// `None` if the run is empty or is not valid UTF-8 - neither can be part of
/// a URL Knot recognizes.
fn take_while(input: &[u8], predicate: impl Fn(u8) -> bool) -> Option<(&str, usize)> {
    let taken = input.iter().take_while(|&&byte| predicate(byte)).count();
    if taken == 0 {
        return None;
    }
    Some((str::from_utf8(&input[..taken]).ok()?, taken))
}

fn take_byte(input: &[u8], byte: u8) -> Option<usize> {
    (input.first() == Some(&byte)).then_some(1)
}

fn is_host_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.')
}

fn is_owner_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

fn is_repo_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}

/// Whether `byte` continues the number rather than ending it, which makes
/// what precedes it not a pull request number at all.
fn is_number_tail_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}

#[cfg(test)]
mod tests;
