//! What the lookup completes over, and how a filter narrows it.
//!
//! Matching is a property of the **source**, not of the entry. A few dozen
//! commands want a case-insensitive substring and no ranking at all - the
//! order they were declared in is meaningful. Thousands of file paths want
//! a subsequence and a score, because `kgs` has to reach
//! `crates/knot-git/src/lib.rs` and the best answer has to come first.
//! Those are different jobs, so the source says which it needs.

use crate::panel_commands::fuzzy;

/// One completable entry: what gets inserted, and what it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LookupEntry {
    /// The token, without its trigger character - the lookup adds that
    /// back on insertion, so a source never has to spell it.
    pub(crate) token:       String,
    /// One line saying what the entry is, shown beside the token.
    pub(crate) description: String,
}

impl LookupEntry {
    pub(crate) fn new(token: impl Into<String>, description: impl Into<String>) -> Self {
        Self { token:       token.into(),
               description: description.into(), }
    }
}

/// One entry a filter selected, and where in its token the filter matched.
///
/// `matched` exists so a row can mark the characters that put it in the
/// list - `panel-file-mentions` requires the user be able to see why a row
/// is a match. A substring source leaves it empty; there is nothing
/// surprising to explain about a contiguous hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LookupMatch {
    pub(crate) entry:   LookupEntry,
    /// Byte offsets into `entry.token`, ascending.
    pub(crate) matched: Vec<usize>,
}

impl LookupMatch {
    /// A match with nothing marked.
    pub(crate) fn plain(entry: LookupEntry) -> Self {
        Self { entry,
               matched: Vec::new() }
    }
}

/// How a source's entries are narrowed and ordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Matcher {
    /// Case-insensitive substring over token *and* description, kept in
    /// registry order. No fuzzy ranking, deliberately: with a few dozen
    /// commands the declared order carries meaning that a score would
    /// destroy.
    Substring,
    /// Case-insensitive subsequence over the token, best first.
    Subsequence,
}

impl Matcher {
    /// The entries `filter` selects, in this matcher's order.
    pub(crate) fn matching(self, entries: &[LookupEntry], filter: &str) -> Vec<LookupMatch> {
        if filter.is_empty() {
            return entries.iter().cloned().map(LookupMatch::plain).collect();
        }
        match self {
            Self::Substring => {
                let filter = filter.to_lowercase();
                entries.iter()
                       .filter(|entry| {
                           entry.token.to_lowercase().contains(&filter)
                           || entry.description.to_lowercase().contains(&filter)
                       })
                       .cloned()
                       .map(LookupMatch::plain)
                       .collect()
            }
            Self::Subsequence => fuzzy::matching(entries, filter),
        }
    }
}

/// A place entries come from. A third source later is a new implementor,
/// not a change to the popup - which is the whole point of the trait.
pub(crate) trait LookupSource {
    fn entries(&self) -> Vec<LookupEntry>;

    /// How this source's entries are narrowed. Commands take the default;
    /// a source of thousands of paths overrides it.
    fn matcher(&self) -> Matcher {
        Matcher::Substring
    }
}
