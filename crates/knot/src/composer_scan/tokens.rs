//! Slash tokens and `@` mentions.
//!
//! Both are "a trigger character, then everything up to whitespace", and
//! both are styled from the trigger onwards before the rest of the token
//! exists - the composer never waits to see whether a token resolves
//! (`panel-rich-input`, "A token being typed is styled as it grows").
//!
//! What separates a token from prose is only where the trigger sits:
//!
//! - `/` has to be the first non-whitespace character of its line, which is the
//!   rule [`crate::panel_commands::active_token`] already applies, so
//!   `crates/knot/src` is a path and not three tokens.
//! - `@` has to follow whitespace or begin the buffer, so the `@` in
//!   `paul@example.com` is part of an address.

use std::ops::Range;

use crate::composer_scan::scan::Construct;
use crate::composer_scan::scan::Span;

/// Every token whose *line* lies within `range`.
///
/// Scanning by line rather than by byte is what makes the incremental path
/// safe: a token cannot span a newline, so a line's tokens depend on that
/// line alone.
pub(super) fn scan(text: &str, range: Range<usize>) -> Vec<Span> {
    let mut spans = Vec::new();
    for (start, line) in super::markdown::lines_in(text, range) {
        scan_slash(line, start, &mut spans);
        scan_mentions(line, start, &mut spans);
    }
    spans
}

/// The leading `/command`, if this line has one.
fn scan_slash(line: &str, start: usize, spans: &mut Vec<Span>) {
    let indent = line.len() - line.trim_start().len();
    if !line[indent..].starts_with('/') {
        return;
    }
    let slash = start + indent;
    let end = token_end(line, indent, start);
    spans.push(Span::new(slash..end, Construct::SlashToken));
}

/// Every `@mention` on this line.
fn scan_mentions(line: &str, start: usize, spans: &mut Vec<Span>) {
    for (offset, _) in line.match_indices('@') {
        if !at_word_boundary(line, offset) {
            continue;
        }
        let end = token_end(line, offset, start);
        // A bare `@` with nothing after it is still a token: the trigger
        // character carries the treatment before the rest is typed.
        spans.push(Span::new(start + offset..end, Construct::Mention));
    }
}

/// Whether the trigger at `offset` begins a word - nothing before it, or
/// whitespace.
fn at_word_boundary(line: &str, offset: usize) -> bool {
    line[..offset].chars()
                  .next_back()
                  .is_none_or(char::is_whitespace)
}

/// The buffer offset one past the token that starts at `offset` in `line`.
fn token_end(line: &str, offset: usize, start: usize) -> usize {
    let after_trigger = offset + 1;
    let rest = &line[after_trigger..];
    let length = rest.find(char::is_whitespace).unwrap_or(rest.len());
    start + after_trigger + length
}
