//! Subsequence matching with a score, for completing file paths.
//!
//! `panel-file-mentions` asks for three things, and the score is built
//! from exactly those: the filter matches as a **subsequence** rather than
//! a substring, so `kgs` reaches `crates/knot-git/src/lib.rs`; matches on
//! the file's **name** outrank matches on its parent directories; and
//! **consecutive** matched characters outrank scattered ones.
//!
//! It is not a general fuzzy finder and does not try to be. There is no
//! learning, no frecency and no typo tolerance - a missing character means
//! no match, which is what keeps a list of thousands honest.

use crate::panel_commands::entry::LookupEntry;
use crate::panel_commands::entry::LookupMatch;

/// Each matched character is worth this before any bonus.
const MATCH: i32 = 1;
/// A character matched immediately after the previous one. Runs are what
/// make a match legible, so this is the largest single bonus.
const CONSECUTIVE: i32 = 8;
/// A character matched inside the file's name rather than in a directory
/// above it.
const IN_NAME: i32 = 6;
/// A character matched at the start of a path segment - the beginning of a
/// word, which reads as intentional rather than incidental.
const SEGMENT_START: i32 = 4;

/// The entries `filter` selects, best first.
pub(crate) fn matching(entries: &[LookupEntry], filter: &str) -> Vec<LookupMatch> {
    let mut scored: Vec<(i32, LookupMatch)> =
        entries.iter()
               .filter_map(|entry| {
                   score(&entry.token, filter).map(|(score, matched)| {
                                                  (score,
                                                   LookupMatch { entry: entry.clone(),
                                                                 matched,
                                                                 is_status: false })
                                              })
               })
               .collect();

    // Score first, then the shorter path, then the path itself. The last
    // two are not quality judgements - they make the order total, so the
    // same filter over the same files always lists the same way.
    scored.sort_by(|(left_score, left), (right_score, right)| {
              right_score.cmp(left_score)
                         .then(left.entry.token.len().cmp(&right.entry.token.len()))
                         .then(left.entry.token.cmp(&right.entry.token))
          });
    scored.into_iter().map(|(_, matched)| matched).collect()
}

/// `filter`'s score against `token`, and the byte offsets it matched, or
/// `None` when `filter` is not a subsequence of it.
///
/// Greedy left to right. A greedy walk can pick an earlier occurrence of a
/// character where a later one would have scored better, which costs this
/// the occasional ideal ordering and saves it from being quadratic on
/// every keystroke over thousands of paths.
fn score(token: &str, filter: &str) -> Option<(i32, Vec<usize>)> {
    let name_start = token.rfind('/').map_or(0, |index| index + 1);

    let mut matched = Vec::new();
    let mut score = 0;
    let mut previous: Option<usize> = None;
    let mut haystack = token.char_indices().peekable();

    for wanted in filter.chars().flat_map(char::to_lowercase) {
        loop {
            let (offset, candidate) = haystack.next()?;
            if !candidate.to_lowercase().eq(std::iter::once(wanted)) {
                continue;
            }
            score += MATCH;
            if previous.is_some_and(|last| last + candidate.len_utf8() == offset) {
                score += CONSECUTIVE;
            }
            if offset >= name_start {
                score += IN_NAME;
            }
            if is_segment_start(token, offset) {
                score += SEGMENT_START;
            }
            matched.push(offset);
            previous = Some(offset);
            break;
        }
    }
    Some((score, matched))
}

/// Whether `offset` begins a path segment or a word within one.
fn is_segment_start(token: &str, offset: usize) -> bool {
    if offset == 0 {
        return true;
    }
    token[..offset].chars()
                   .next_back()
                   .is_some_and(|previous| matches!(previous, '/' | '-' | '_' | '.' | ' '))
}
