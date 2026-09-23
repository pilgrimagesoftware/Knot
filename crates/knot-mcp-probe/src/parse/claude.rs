//! Reading `claude mcp list`.
//!
//! There is no `--json`. The output is written for people, and the parser is
//! therefore pinned to a format that can change under it - which is why the
//! fixtures beside this file are the contract, and why every failure mode
//! here degrades rather than throwing the listing away.
//!
//! The line shape is:
//!
//! ```text
//! <name>: <target> - <glyph> <state text>
//! ```
//!
//! and the only safe way to split it is **from the right, anchored on the
//! state**. The target is unbounded and contains ` - ` freely: one observed
//! entry was a 1.5 KB inline `node -e` program with several. Splitting on the
//! first ` - `, or on any ` - ` without checking what follows it, cuts that
//! entry in half.

use crate::server::{ServerRow, Target};
use crate::state::ServerState;

/// The separator between a server's target and its state.
const STATE_SEPARATOR: &str = " - ";

/// The separator between a name and everything after it.
const NAME_SEPARATOR: &str = ": ";

/// Transport markers the CLI appends to a target.
const TRANSPORT_MARKERS: &[&str] = &["(HTTP)", "(SSE)", "(http)", "(sse)"];

/// Parses one listing into rows.
///
/// Returns `None` when no line in the output looks like a server at all -
/// the caller turns that into [`crate::ProbeError::Unrecognized`], because
/// output the parser cannot read means the format moved, not that the agent
/// has no servers.
pub(crate) fn parse(output: &str) -> Option<Vec<ServerRow>> {
    let candidates: Vec<&str> = output.lines()
                                      .map(str::trim)
                                      .filter(|line| line.contains(NAME_SEPARATOR))
                                      .collect();

    // A listing is recognizable when at least one line carries a state we
    // know. Without that anchor the "candidates" are just lines with a colon
    // in them - a usage message, an error, a prompt - and calling those
    // servers would invent rows out of a failure.
    let recognizable = candidates.iter().any(|line| split_state(line).is_some());
    if !recognizable {
        return None;
    }

    Some(candidates.iter().map(|line| row_from(line)).collect())
}

/// One line as a row.
///
/// A candidate with no recognizable state still becomes a row, in
/// [`ServerState::Unknown`]. Dropping it would shorten the list silently,
/// and a server Knot can see but cannot classify is worth saying out loud.
fn row_from(line: &str) -> ServerRow {
    // A line whose state we cannot name still has the shape `name: target -
    // state`, so fall back to the last separator. That keeps the unreadable
    // text out of the target and puts it where the row can show it, which is
    // the whole point of degrading rather than dropping.
    let (head, state_text) = split_state(line).or_else(|| line.rsplit_once(STATE_SEPARATOR))
                                              .unwrap_or((line, ""));

    let (name, target_text) = head.split_once(NAME_SEPARATOR).unwrap_or((head, ""));
    let (state, detail) = classify(state_text);

    let row = ServerRow::new(name.trim(), target_of(target_text), state);

    match detail {
        Some(detail) => row.with_detail(detail),
        None => row,
    }
}

/// Splits a line into everything before its state and the state text.
///
/// Scans the ` - ` occurrences from the right and takes the first one whose
/// tail names a state. Right-anchored because the *target* is what may hold
/// the separator; the state never does.
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
fn named_state(text: &str) -> Option<ServerState> {
    let trimmed = text.trim();
    let lowered = trimmed.to_lowercase();

    // Glyph first - it is what the CLI actually keys on - with the wording as
    // a fallback so losing the glyph alone does not lose the state.
    if trimmed.starts_with('✔') || lowered.starts_with("connected") {
        return Some(ServerState::Connected);
    }
    if trimmed.starts_with('⊘') || lowered.contains("disabled") {
        return Some(ServerState::Disabled);
    }
    if trimmed.starts_with('⏸') || lowered.contains("pending approval") {
        return Some(ServerState::PendingApproval);
    }
    if lowered.contains("needs authentication")
       || lowered.contains("not authenticated")
       || lowered.contains("authentication required")
    {
        return Some(ServerState::NeedsAuthentication);
    }
    if trimmed.starts_with('✘') || lowered.contains("failed to connect") {
        return Some(ServerState::Failed);
    }

    None
}

/// The state and the reason behind it.
///
/// Authentication is checked before failure on purpose: the CLI marks a
/// server needing authentication with the same ✘ it uses for a connection
/// failure, and reporting "failed" there would send the user to repair
/// something that is merely locked.
fn classify(text: &str) -> (ServerState, Option<String>) {
    let Some(state) = named_state(text)
    else {
        return (ServerState::Unknown, unknown_detail(text));
    };

    (state, detail_of(text))
}

/// The reason, when the CLI gave one after an em dash or a colon.
fn detail_of(text: &str) -> Option<String> {
    let without_glyph = text.trim().trim_start_matches(['✔', '✘', '⊘', '⏸']).trim();

    let detail = without_glyph.split_once('—')
                              .map(|(_, rest)| rest)
                              .or_else(|| without_glyph.split_once(" - ").map(|(_, rest)| rest))?;

    let detail = detail.trim();

    (!detail.is_empty()).then(|| detail.to_owned())
}

/// For a state we could not name, the text itself is the only thing worth
/// showing - it is what the CLI said and we could not read.
fn unknown_detail(text: &str) -> Option<String> {
    let trimmed = text.trim();

    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

/// A target as a URL or a command, with the transport marker stripped.
fn target_of(text: &str) -> Target {
    let mut trimmed = text.trim();

    for marker in TRANSPORT_MARKERS {
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
