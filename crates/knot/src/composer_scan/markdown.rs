//! Markdown as the composer sees it: enough to tell one construct from
//! another while it is being typed, and no more.
//!
//! This is deliberately not a second CommonMark implementation, and it is
//! not `markdown_view.rs`. That path turns a *finished* document into
//! rendered elements; this one turns a *half-typed* buffer into ranges,
//! incrementally, on every keystroke. They disagree at the edges by
//! design, and the proposal records that a composer which styles slightly
//! more eagerly than the renderer is a far smaller harm than a per-
//! keystroke block parse.
//!
//! Two rules shape everything here:
//!
//! - **A construct that has not closed is prose.** A lone `*` while typing must
//!   not italicise the rest of the line, so an opener with no closer
//!   contributes nothing. The one exception is a fence, which the spec says
//!   styles what follows it.
//! - **Only fences cross lines.** Emphasis, code spans, links, headings, quotes
//!   and list markers are decided by their own line, which is what bounds the
//!   incremental path: widen an edit to its line, and to the enclosing fence,
//!   and nothing outside can have changed.

use std::ops::Range;

use crate::composer_scan::scan::Construct;
use crate::composer_scan::scan::Span;

/// The lines of `text` that intersect `range`, as `(start offset, line)`
/// with the newline excluded.
///
/// `range` is widened to whole lines: a caller asking about the middle of
/// a line is asking about that line.
pub(super) fn lines_in(text: &str, range: Range<usize>) -> Vec<(usize, &str)> {
    let start = line_start(text, range.start.min(text.len()));
    let end = line_end(text, range.end.min(text.len()));
    if start > end {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut offset = start;
    for line in text[start..end].split('\n') {
        lines.push((offset, line));
        offset += line.len() + 1;
    }
    lines
}

/// The offset of the start of the line containing `offset`.
pub(super) fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |index| index + 1)
}

/// The offset of the end of the line containing `offset`, newline
/// excluded.
pub(super) fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n')
                  .map_or(text.len(), |index| offset + index)
}

/// Every markdown construct whose line lies within `range`.
///
/// Fenced blocks are resolved over the whole buffer first, because whether
/// a line is code depends on a delimiter that may be far above it. Only
/// the delimiter test runs over the whole buffer - a few byte comparisons
/// per line - while the inline scanning, which is the expensive half, runs
/// only over `range`.
pub(super) fn scan(text: &str, range: Range<usize>) -> Vec<Span> {
    let fences = fenced_blocks(text);
    let mut spans: Vec<Span> =
        fences.iter()
              .filter(|block| block.start < range.end.max(range.start) && range.start <= block.end)
              .map(|block| Span::new(block.clone(), Construct::CodeFence))
              .collect();

    for (start, line) in lines_in(text, range) {
        if fences.iter().any(|block| block.contains(&start)) {
            continue;
        }
        scan_line(line, start, &mut spans);
    }
    spans
}

/// The fenced blocks of `text`, each covering its opening fence through
/// its closing one.
///
/// An unclosed fence runs to the end of the buffer: the spec asks for the
/// line after an opening fence to be code even though the block is still
/// being typed, which is the one place an unfinished construct still
/// styles.
fn fenced_blocks(text: &str) -> Vec<Range<usize>> {
    let mut blocks = Vec::new();
    let mut open: Option<(usize, char, usize)> = None;

    let mut offset = 0;
    for line in text.split('\n') {
        let line_span = offset..offset + line.len();
        offset += line.len() + 1;

        match open {
            None => {
                if let Some((marker, length)) = fence(line) {
                    open = Some((line_span.start, marker, length));
                }
            }
            Some((block_start, marker, length)) => {
                // A closing fence is the same character, at least as long,
                // and carries no info string.
                if let Some((closing, closing_length)) = fence(line)
                   && closing == marker
                   && closing_length >= length
                   && line.trim_end().trim_start_matches(marker).trim().is_empty()
                {
                    blocks.push(block_start..line_span.end);
                    open = None;
                }
            }
        }
    }
    if let Some((block_start, _, _)) = open {
        blocks.push(block_start..text.len());
    }
    blocks
}

/// The fence marker and its run length, if `line` is a fence delimiter.
///
/// The info string is read only to find where the run ends: a `rust` block
/// and a `python` block are the same construct, because a composer is for
/// writing a request and a language-aware highlighter is a cost paid on
/// every keystroke for a few quoted lines (`panel-rich-input`, "Fenced
/// code is not highlighted by language").
fn fence(line: &str) -> Option<(char, usize)> {
    let indent = line.len() - line.trim_start().len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let marker = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let length = rest.chars().take_while(|c| *c == marker).count();
    (length >= 3).then_some((marker, length))
}

/// The constructs on one line that is not inside a fence.
fn scan_line(line: &str, start: usize, spans: &mut Vec<Span>) {
    if line.trim().is_empty() {
        return;
    }
    let indent = line.len() - line.trim_start().len();
    if indent > 3 {
        // An indented code block. One code treatment, like a fence.
        spans.push(Span::new(start..start + line.len(), Construct::CodeFence));
        return;
    }
    let body = &line[indent..];

    if let Some(level) = heading_level(body) {
        let _ = level;
        spans.push(Span::new(start..start + line.len(), Construct::Heading));
        // A heading's own text can still hold a code span or a link.
        scan_inline(body, start + indent, spans);
        return;
    }

    if body.starts_with('>') {
        spans.push(Span::new(start..start + line.len(), Construct::BlockQuote));
        let quoted = body.trim_start_matches('>');
        let offset = start + indent + (body.len() - quoted.len());
        scan_inline(quoted, offset, spans);
        return;
    }

    let after_marker = match list_marker(body) {
        Some(length) => {
            spans.push(Span::new(start + indent..start + indent + length,
                                 Construct::ListMarker));
            length
        }
        None => 0,
    };
    scan_inline(&body[after_marker..], start + indent + after_marker, spans);
}

/// The heading level of `body`, if it is an ATX heading.
fn heading_level(body: &str) -> Option<usize> {
    let hashes = body.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    // `#hashtag` is not a heading: the run has to be followed by a space
    // or be the whole line.
    let rest = &body[hashes..];
    (rest.is_empty() || rest.starts_with(' ')).then_some(hashes)
}

/// The length of the list marker at the head of `body`, including the
/// space that follows it.
fn list_marker(body: &str) -> Option<usize> {
    let bullet = body.starts_with("- ") || body.starts_with("* ") || body.starts_with("+ ");
    if bullet {
        return Some(2);
    }
    let digits = body.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = &body[digits..];
    if rest.starts_with(". ") || rest.starts_with(") ") {
        return Some(digits + 2);
    }
    None
}

/// Code spans, links and emphasis within one line's text.
///
/// Order is precedence: a code span swallows whatever is inside it, a link
/// is read before the emphasis markers its text might contain, and strong
/// is read before emphasis so `**x**` is not two emphases.
fn scan_inline(text: &str, start: usize, spans: &mut Vec<Span>) {
    let mut taken = vec![false; text.len()];

    scan_code_spans(text, start, &mut taken, spans);
    scan_links(text, start, &mut taken, spans);
    scan_delimited(text, start, "**", Construct::Strong, &mut taken, spans);
    scan_delimited(text, start, "__", Construct::Strong, &mut taken, spans);
    scan_delimited(text, start, "*", Construct::Emphasis, &mut taken, spans);
    scan_delimited(text, start, "_", Construct::Emphasis, &mut taken, spans);
}

/// Whether every byte of `range` is still unclaimed.
fn free(taken: &[bool], range: Range<usize>) -> bool {
    taken.get(range)
         .is_some_and(|slice| !slice.iter().any(|byte| *byte))
}

/// Claim `range` so a later, lower-precedence pass skips it.
fn claim(taken: &mut [bool], range: Range<usize>) {
    if let Some(slice) = taken.get_mut(range) {
        slice.fill(true);
    }
}

/// `` `code` ``, including runs of several backticks.
fn scan_code_spans(text: &str, start: usize, taken: &mut [bool], spans: &mut Vec<Span>) {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let run = bytes[index..].iter()
                                .take_while(|byte| **byte == b'`')
                                .count();
        let after = index + run;
        // The closing run has to be exactly as long, so `` ` `` inside a
        // double-backtick span stays literal.
        let mut cursor = after;
        let mut closed = None;
        while cursor < bytes.len() {
            if bytes[cursor] != b'`' {
                cursor += 1;
                continue;
            }
            let closing = bytes[cursor..].iter()
                                         .take_while(|byte| **byte == b'`')
                                         .count();
            if closing == run {
                closed = Some(cursor + closing);
                break;
            }
            cursor += closing;
        }
        match closed {
            Some(end) => {
                spans.push(Span::new(start + index..start + end, Construct::InlineCode));
                claim(taken, index..end);
                index = end;
            }
            // Unclosed: prose, and the run is skipped so it cannot open
            // again halfway through itself.
            None => index = after,
        }
    }
}

/// `[text](target)`.
fn scan_links(text: &str, start: usize, taken: &mut [bool], spans: &mut Vec<Span>) {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'[' || !free(taken, index..index + 1) {
            index += 1;
            continue;
        }
        let Some(close) = bytes[index..].iter().position(|byte| *byte == b']')
        else {
            return;
        };
        let close = index + close;
        if bytes.get(close + 1) != Some(&b'(') {
            index += 1;
            continue;
        }
        let Some(paren) = bytes[close..].iter().position(|byte| *byte == b')')
        else {
            return;
        };
        let end = close + paren + 1;
        if free(taken, index..end) {
            spans.push(Span::new(start + index..start + end, Construct::Link));
            claim(taken, index..end);
        }
        index = end;
    }
}

/// A construct delimited by the same marker on both sides, such as `*` or
/// `**`.
///
/// The delimiters follow the usual flanking rule, which is what keeps
/// `a * b` prose: an opener is not followed by whitespace, and a closer is
/// not preceded by it.
fn scan_delimited(text: &str, start: usize, marker: &str, construct: Construct,
                  taken: &mut [bool], spans: &mut Vec<Span>) {
    let mut index = 0;
    while let Some(found) = text[index..].find(marker) {
        let open = index + found;
        let after_open = open + marker.len();
        if !free(taken, open..after_open) || !opens(text, after_open, marker) {
            index = after_open.min(text.len());
            continue;
        }
        let Some(close) = find_closer(text, after_open, marker, taken)
        else {
            // No closer: this opener is prose, and so is everything after
            // it on this line as far as this marker is concerned.
            return;
        };
        let end = close + marker.len();
        spans.push(Span::new(start + open..start + end, construct));
        claim(taken, open..end);
        index = end;
    }
}

/// Whether a delimiter at `after_open` can open: something follows it, and
/// that something is not whitespace or another copy of the marker.
fn opens(text: &str, after_open: usize, marker: &str) -> bool {
    let rest = &text[after_open..];
    rest.chars()
        .next()
        .is_some_and(|c| !c.is_whitespace() && !marker.starts_with(c))
}

/// The offset of the closing delimiter for an opener ending at `from`.
fn find_closer(text: &str, from: usize, marker: &str, taken: &[bool]) -> Option<usize> {
    let mut index = from;
    while let Some(found) = text[index..].find(marker) {
        let close = index + found;
        if close > from
           && free(taken, close..close + marker.len())
           && text[..close].chars()
                           .next_back()
                           .is_some_and(|c| !c.is_whitespace())
        {
            return Some(close);
        }
        index = close + marker.len();
        if index >= text.len() {
            break;
        }
    }
    None
}
