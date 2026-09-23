//! How much of the buffer an edit can have changed.
//!
//! The composer re-renders per keystroke, so the scanner's cost has to be
//! a function of what was edited rather than of how much has been typed
//! (`panel-rich-input`, "Styling keeps up with typing"). This is where
//! that bound is decided.
//!
//! The widening follows what actually crosses a line:
//!
//! 1. Start from the bytes the edit touched, in the text after it.
//! 2. Widen to whole lines. Emphasis, code spans, links, headings, quotes and
//!    list markers are decided by their own line, so this alone covers all of
//!    them.
//! 3. Widen to the enclosing fenced block, since a line inside one is code
//!    regardless of what it says.
//! 4. If the edit could have opened or closed a fence, widen to the end of the
//!    buffer - toggling a delimiter re-reads every line after it, and there is
//!    no cheaper honest answer.
//!
//! Step 4 is the expensive case and it is also the rare one: it needs the
//! edit to have touched a fence delimiter. Typing into a paragraph, which
//! is what typing mostly is, stops at step 2.

use std::ops::Range;

use crate::composer_scan::markdown;
use crate::composer_scan::scan::Edit;

/// The range of `text` - the buffer *after* `edit` - that has to be
/// scanned again.
pub(crate) fn dirty_range(text: &str, edit: &Edit) -> Range<usize> {
    let touched = edit.inserted_range();
    let start = markdown::line_start(text, touched.start.min(text.len()));
    let end = markdown::line_end(text, touched.end.min(text.len()));

    if touches_fence(text, start..end) {
        return 0..text.len();
    }

    enclosing_fence(text, start..end).unwrap_or(start..end)
}

/// Whether any line in `range` is a fence delimiter.
///
/// Asked of the text after the edit only. A delimiter the edit *removed*
/// is covered by the same test on the line that replaced it: the line is
/// in `range` either way, and if it stopped being a delimiter the block it
/// opened has changed, which this still has to catch. The conservative
/// answer - widen whenever the edited lines look like a fence now, or did
/// before - costs a full rescan on an edit to a fence line, which is the
/// case step 4 exists for.
fn touches_fence(text: &str, range: Range<usize>) -> bool {
    markdown::lines_in(text, range).iter()
                                   .any(|(_, line)| is_fence(line))
}

/// Whether `line` is a fence delimiter, by the same test the scanner uses.
fn is_fence(line: &str) -> bool {
    let indent = line.len() - line.trim_start().len();
    if indent > 3 {
        return false;
    }
    let rest = &line[indent..];
    let Some(marker) = rest.chars().next().filter(|c| *c == '`' || *c == '~')
    else {
        return false;
    };
    rest.chars().take_while(|c| *c == marker).count() >= 3
}

/// The fenced block containing `range`, if it is inside one.
///
/// Found by counting delimiters from the top of the buffer, which is a
/// handful of byte comparisons per line - not the inline scanning that
/// `range` exists to bound.
fn enclosing_fence(text: &str, range: Range<usize>) -> Option<Range<usize>> {
    let mut open: Option<usize> = None;
    let mut offset = 0;

    for line in text.split('\n') {
        let line_span = offset..offset + line.len();
        offset += line.len() + 1;

        match open {
            None => {
                if is_fence(line) {
                    open = Some(line_span.start);
                }
            }
            Some(block_start) => {
                if is_fence(line) {
                    let block = block_start..line_span.end;
                    if block.start <= range.start && range.end <= block.end {
                        return Some(block);
                    }
                    open = None;
                }
            }
        }
    }

    // An unclosed fence runs to the end of the buffer.
    open.filter(|block_start| *block_start <= range.start)
        .map(|block_start| block_start..text.len())
}
