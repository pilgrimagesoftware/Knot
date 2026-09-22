//! Finding the slash token the caret sits in.
//!
//! A token is active when the first non-whitespace character of the caret's
//! line is `/` and the caret has not moved past the token's end. A `/` in
//! the middle of prose is just a slash - the lookup stays shut, per the
//! design decision "The active token is a leading slash on the caret's
//! line".

use std::ops::Range;

/// The slash token under the caret: where it sits in the buffer, and what
/// has been typed into it so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveToken {
    /// Byte range of the token *including* its leading `/`, so inserting an
    /// entry can replace exactly this span.
    pub(crate) range:  Range<usize>,
    /// The text after the `/`, which the lookup filters on. Empty when the
    /// user has typed only `/`.
    pub(crate) filter: String,
}

/// The active token in `value` for a caret at byte offset `caret`, or
/// `None` when the caret is not in one.
pub(crate) fn active_token(value: &str, caret: usize) -> Option<ActiveToken> {
    if caret > value.len() || !value.is_char_boundary(caret) {
        return None;
    }
    let line_start = value[..caret].rfind('\n').map_or(0, |index| index + 1);
    let line_end = value[caret..].find('\n')
                                 .map_or(value.len(), |index| caret + index);
    let line = &value[line_start..line_end];

    let indent = line.len() - line.trim_start().len();
    let slash = line_start + indent;
    if !line[indent..].starts_with('/') {
        return None;
    }

    let after_slash = slash + 1;
    let token_end = value[after_slash..line_end].find(char::is_whitespace)
                                                .map_or(line_end, |index| after_slash + index);
    // A caret past the token's end is editing the words after it, not the
    // token - the lookup has no business being open there.
    if caret < slash || caret > token_end {
        return None;
    }

    Some(ActiveToken { range:  slash..token_end,
                       filter: value[after_slash..token_end].to_string(), })
}
