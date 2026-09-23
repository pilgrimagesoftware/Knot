//! Finding the token the caret sits in, and which trigger opened it.
//!
//! Two triggers reach the same popup: `/` completes commands and skills,
//! `@` completes files from the agent's folder. Which one is live is not a
//! pair of flags that could both be set - it is whichever token the caret
//! is inside, so the two are mutually exclusive by construction.
//!
//! What counts as a token is **not** decided here. It is
//! [`crate::composer_scan`]'s answer, the same one that decides what gets
//! the token treatment on screen, so the lookup cannot open on a run the
//! composer draws as prose or vice versa - which is what
//! `panel-slash-commands`' "an inserted command is styled as a token"
//! requires. This module only asks which of those runs the caret is in.

use std::ops::Range;

use crate::composer_scan::Construct;
use crate::composer_scan::tokens_on_line;

/// Which character opened the token.
///
/// A closed set, so an enum: the popup dispatches on it, and a `String`
/// here would need a `_ =>` arm that could only guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Trigger {
    /// `/` - knot's commands and the agent's skills.
    Slash,
    /// `@` - files in the agent's working folder.
    Mention,
}

impl Trigger {
    /// The character that opens this trigger, which insertion writes back.
    pub(crate) fn char(self) -> char {
        match self {
            Self::Slash => '/',
            Self::Mention => '@',
        }
    }

    /// The trigger a scanned construct represents, if it is a token at all.
    fn of(construct: Construct) -> Option<Self> {
        match construct {
            Construct::SlashToken => Some(Self::Slash),
            Construct::Mention => Some(Self::Mention),
            _ => None,
        }
    }
}

/// The token under the caret: where it sits, what has been typed into it,
/// and which trigger it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActiveToken {
    /// Byte range of the token *including* its trigger character, so
    /// inserting an entry can replace exactly this span.
    pub(crate) range:   Range<usize>,
    /// The text after the trigger, which the lookup filters on. Empty when
    /// only the trigger has been typed.
    pub(crate) filter:  String,
    pub(crate) trigger: Trigger,
}

/// The active token in `value` for a caret at byte offset `caret`, or
/// `None` when the caret is not in one.
///
/// A caret past a token's end is editing the words after it, not the
/// token, and the lookup has no business being open there.
pub(crate) fn active_token(value: &str, caret: usize) -> Option<ActiveToken> {
    if caret > value.len() || !value.is_char_boundary(caret) {
        return None;
    }
    tokens_on_line(value, caret).into_iter()
                                .find(|span| span.range.start <= caret && caret <= span.range.end)
                                .and_then(|span| {
                                    let trigger = Trigger::of(span.construct)?;
                                    let after_trigger =
                                        span.range.start + trigger.char().len_utf8();
                                    Some(ActiveToken { filter: value[after_trigger..span.range.end]
                                                           .to_string(),
                                                       range: span.range,
                                                       trigger })
                                })
}
