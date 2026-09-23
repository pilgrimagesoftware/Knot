//! The grammar every agent CLI's listing turns out to share.
//!
//! Claude Code and Gemini both print
//!
//! ```text
//! [glyph ]<name>: <target>[ (transport)] - <state>
//! ```
//!
//! and differ only in which glyphs they use, which transport markers they
//! append, and how they say "nothing is configured". So the grammar lives
//! here once and each format contributes a [`FormatSpec`] rather than its own
//! parser - one place to fix when the shape itself is wrong, which it has
//! already been once.
//!
//! Two rules do the work, and both exist because of a real line:
//!
//! - **Split from the right, anchored on the state.** The target is unbounded
//!   and contains ` - ` freely; one observed entry was a 1.5 KB inline `node
//!   -e` program with three of them.
//! - **An entry needs both separators.** Gemini prefixes its listing with
//!   `Warning: MCP servers are configured but disabled...`, which has the
//!   `name: value` shape and would otherwise become a server called "Warning".

use crate::server::{ServerRow, Target};
use crate::state::ServerState;

/// The separator between a server's target and its state.
const STATE_SEPARATOR: &str = " - ";

/// The separator between a name and everything after it.
const NAME_SEPARATOR: &str = ": ";

/// What distinguishes one CLI's listing from another's.
pub(crate) struct FormatSpec {
    /// Status glyphs stripped from the start of an entry before its name.
    pub(crate) leading_glyphs:    &'static [char],
    /// Transport markers stripped from the end of a target.
    pub(crate) transport_markers: &'static [&'static str],
    /// Phrases that mean "this agent has no MCP servers configured".
    ///
    /// Without these an empty configuration is indistinguishable from output
    /// the parser cannot read, and the section would say "could not read
    /// this" to a user whose agent simply has no servers.
    pub(crate) empty_markers:     &'static [&'static str],
}

/// Reads a listing, or `None` when the output is not one.
pub(crate) fn parse(output: &str, spec: &FormatSpec) -> Option<Vec<ServerRow>> {
    let candidates: Vec<&str> = output.lines()
                                      .map(str::trim)
                                      .filter(|line| is_entry(line))
                                      .collect();

    if candidates.is_empty() {
        // No entry, but the CLI may have said outright that there are none.
        // That is an answer, and a different one from "unreadable".
        return says_empty(output, spec).then(Vec::new);
    }

    // At least one entry must carry a state we know. Without that anchor the
    // candidates are just lines with the right punctuation, and calling them
    // servers would invent rows out of a failure.
    let recognizable = candidates.iter().any(|line| split_state(line).is_some());
    if !recognizable {
        return None;
    }

    Some(candidates.iter().map(|line| row_from(line, spec)).collect())
}

/// Whether a line has the shape of an entry.
///
/// Both separators, deliberately. A name and a colon alone matches every
/// warning and usage line a CLI prints.
fn is_entry(line: &str) -> bool {
    line.contains(NAME_SEPARATOR) && line.contains(STATE_SEPARATOR)
}

/// Whether the output says in words that nothing is configured.
fn says_empty(output: &str, spec: &FormatSpec) -> bool {
    let lowered = output.to_lowercase();

    spec.empty_markers
        .iter()
        .any(|marker| lowered.contains(&marker.to_lowercase()))
}

/// One line as a row.
///
/// An entry whose state we cannot name still becomes a row, in
/// [`ServerState::Unknown`]. Dropping it would shorten the list silently, and
/// a server Knot can see but cannot classify is worth saying out loud.
fn row_from(line: &str, spec: &FormatSpec) -> ServerRow {
    let line = line.trim_start_matches(spec.leading_glyphs).trim_start();

    // Falling back to the last separator keeps unreadable state text out of
    // the target and puts it where the row can show it - the whole point of
    // degrading rather than dropping.
    let (head, state_text) = split_state(line).or_else(|| line.rsplit_once(STATE_SEPARATOR))
                                              .unwrap_or((line, ""));

    let (name, target_text) = head.split_once(NAME_SEPARATOR).unwrap_or((head, ""));
    let (state, detail) = classify(state_text);

    let row = ServerRow::new(name.trim(), target_of(target_text, spec), state);

    match detail {
        Some(detail) => row.with_detail(detail),
        None => row,
    }
}

/// Splits a line into everything before its state and the state text.
///
/// Scans ` - ` from the right and takes the first occurrence whose tail names
/// a state. Right-anchored because the *target* is what may hold the
/// separator; the state never does.
fn split_state(line: &str) -> Option<(&str, &str)> {
    let mut search_end = line.len();

    while let Some(at) = line[..search_end].rfind(STATE_SEPARATOR) {
        let tail = &line[at + STATE_SEPARATOR.len()..];

        if named_state(tail).is_some() {
            return Some((&line[..at], tail));
        }

        search_end = at;
    }

    None
}

/// The state a trailing segment names, if it names one.
///
/// Glyph first - it is what the CLI keys on - with the wording as a fallback,
/// so losing the glyph alone does not lose the state. Gemini has no glyph in
/// this position at all and relies entirely on the wording.
pub(crate) fn named_state(text: &str) -> Option<ServerState> {
    let trimmed = text.trim();
    let lowered = trimmed.to_lowercase();

    if trimmed.starts_with('✔') || lowered.starts_with("connected") || lowered.starts_with("ready")
    {
        return Some(ServerState::Connected);
    }
    if trimmed.starts_with('⊘') || lowered.contains("disabled") {
        return Some(ServerState::Disabled);
    }
    if trimmed.starts_with('⏸') || lowered.contains("pending approval") {
        return Some(ServerState::PendingApproval);
    }

    // Before failure on purpose: a server needing authentication is marked
    // with the same ✘ as one that could not be reached, and telling the user
    // to repair something that is merely locked is the wrong instruction.
    if lowered.contains("needs authentication")
       || lowered.contains("not authenticated")
       || lowered.contains("authentication required")
       || lowered.contains("needs auth")
    {
        return Some(ServerState::NeedsAuthentication);
    }

    if trimmed.starts_with('✘')
       || lowered.contains("failed to connect")
       || lowered.starts_with("disconnected")
    {
        return Some(ServerState::Failed);
    }

    None
}

/// The state and the reason behind it.
fn classify(text: &str) -> (ServerState, Option<String>) {
    let Some(state) = named_state(text)
    else {
        return (ServerState::Unknown, non_empty(text));
    };

    (state, detail_of(text))
}

/// The reason, when the CLI gave one after an em dash or a second separator.
fn detail_of(text: &str) -> Option<String> {
    let without_glyph = text.trim().trim_start_matches(['✔', '✘', '⊘', '⏸']).trim();

    let detail =
        without_glyph.split_once('—')
                     .map(|(_, rest)| rest)
                     .or_else(|| without_glyph.split_once(STATE_SEPARATOR).map(|(_, r)| r))?;

    non_empty(detail)
}

fn non_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// A target as a URL or a command, with the transport marker stripped.
fn target_of(text: &str, spec: &FormatSpec) -> Target {
    let mut trimmed = text.trim();

    for marker in spec.transport_markers {
        if let Some(stripped) = trimmed.strip_suffix(marker) {
            trimmed = stripped.trim_end();
            break;
        }
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Target::Http { url: trimmed.to_owned(), };
    }

    Target::Stdio { command: trimmed.to_owned(), }
}
