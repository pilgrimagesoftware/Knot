//! The incremental path against the whole-buffer one.
//!
//! `rescan` exists only because `scan` is too expensive to run on every
//! keystroke. It is therefore never allowed to disagree with it, and the
//! equivalence tests here are what say so: every edit sequence is applied
//! both ways and the two span sets compared. A disagreement is the bug -
//! the design says as much, and this is where it would show.

use std::ops::Range;

use crate::composer_scan::Edit;
use crate::composer_scan::dirty::dirty_range;
use crate::composer_scan::rescan;
use crate::composer_scan::scan;

/// Applies `text[replaced] = inserted` and returns the new buffer with the
/// edit that produced it.
fn apply(text: &str, replaced: Range<usize>, inserted: &str) -> (String, Edit) {
    let mut next = String::with_capacity(text.len() + inserted.len());
    next.push_str(&text[..replaced.start]);
    next.push_str(inserted);
    next.push_str(&text[replaced.end..]);
    let edit = Edit { replaced,
                      inserted: inserted.len() };
    (next, edit)
}

/// Replays `edits` through both paths, asserting after every step that the
/// incremental result is exactly the whole-buffer one.
fn agree(start: &str, edits: &[(Range<usize>, &str)]) {
    let mut text = start.to_string();
    let mut spans = scan(&text, &[]);
    assert_eq!(spans, scan(&text, &[]), "scan must be deterministic");

    for (step, (replaced, inserted)) in edits.iter().enumerate() {
        let (next, edit) = apply(&text, replaced.clone(), inserted);
        let incremental = rescan(&next, &[], &spans, &edit);
        let whole = scan(&next, &[]);
        assert_eq!(incremental, whole,
                   "step {step}: rescan disagreed with a full scan of {next:?}");
        text = next;
        spans = incremental;
    }
}

#[test]
fn typing_a_word_agrees_with_a_full_scan() {
    agree("",
          &[(0..0, "h"),
            (1..1, "e"),
            (2..2, "l"),
            (3..3, "l"),
            (4..4, "o")]);
}

#[test]
fn closing_an_emphasis_restyles_the_line() {
    agree("a *word", &[(7..7, "*")]);
}

#[test]
fn opening_a_fence_restyles_what_follows() {
    agree("line one\nline two", &[(8..8, "\n```")]);
}

/// The case the design calls out as where the subtle bugs will be: a fence
/// closed far below the edit.
#[test]
fn closing_a_fence_far_below_the_open_agrees() {
    agree("```\none\ntwo\nthree", &[(17..17, "\n```")]);
}

/// Deleting a closing fence reopens the block, so every line after it
/// becomes code. The incremental path has to notice that from the deleted
/// line alone.
#[test]
fn deleting_a_closing_fence_agrees() {
    agree("```\ncode\n```\nafter", &[(9..12, "")]);
}

#[test]
fn pasting_a_document_agrees() {
    agree("intro\n",
          &[(6..6,
             "# Heading\n\n- a list item\n\n```rust\nlet x = 1;\n```\n\nand **strong** \
                    prose with `code`.")]);
}

#[test]
fn a_multi_line_paste_into_the_middle_agrees() {
    agree("before\nafter", &[(7..7, "## Title\n\n> quoted\n\n")]);
}

#[test]
fn cutting_a_construct_out_agrees() {
    agree("keep **this** and this", &[(5..13, "")]);
}

#[test]
fn replacing_a_selection_agrees() {
    agree("see [the spec](a.md) now", &[(4..20, "`code`")]);
}

/// Undo and redo arrive as ordinary edits, so the guard on them is that
/// the buffer returns to a state whose scan matches.
#[test]
fn undoing_a_deletion_agrees() {
    agree("```\ncode\n```", &[(9..12, ""), (9..9, "```")]);
}

#[test]
fn typing_a_token_character_by_character_agrees() {
    agree("look at ",
          &[(8..8, "@"),
            (9..9, "l"),
            (10..10, "i"),
            (11..11, "b"),
            (12..12, ".rs")]);
}

#[test]
fn editing_inside_a_fence_agrees() {
    agree("prose\n```\nlet x = 1;\n```\nmore prose", &[(19..19, "23")]);
}

/// `panel-rich-input`: "An edit inside a fence restyles the fence, not the
/// document", and "Typing into a long prompt" must not slow down.
///
/// The bound asserted is the dirty range, which is what the expensive half
/// of the scanner reads. A scanner changed to re-read everything returns
/// the whole buffer here and fails.
#[test]
fn a_single_character_edit_is_bounded_by_its_construct() {
    let paragraph = "Some ordinary prose about the change.\n";
    let long = paragraph.repeat(400);
    let caret = paragraph.len() * 200;

    let (text, edit) = apply(&long, caret..caret, "x");
    let dirty = dirty_range(&text, &edit);

    assert!(dirty.len() <= paragraph.len(),
            "one character typed into a paragraph should dirty that paragraph's line, not \
             {} bytes of a {} byte buffer",
            dirty.len(),
            text.len());
    assert!(dirty.len() * 50 < text.len(),
            "the bound has to be a construct, not a fraction of the buffer - it would grow with \
             the prompt");
}

/// The same edit inside a fenced block widens to the block, and stops
/// there rather than running to the end of a long document.
#[test]
fn an_edit_inside_a_fence_is_bounded_by_the_fence() {
    let tail = "trailing prose\n".repeat(400);
    let text = format!("intro\n```\nlet x = 1;\nlet y = 2;\n```\n{tail}");
    let caret = text.find("let y")
                    .expect("the fixture has a second line of code");

    let (text, edit) = apply(&text, caret..caret, "z");
    let dirty = dirty_range(&text, &edit);

    assert!(dirty.len() < 60,
            "the fenced block is the bound, but {} bytes were dirtied",
            dirty.len());
    assert!(dirty.end < text.len(),
            "the prose after the block cannot have changed");
}

/// Touching a fence delimiter is the one case that costs a full rescan,
/// and it is worth stating so the bound above is not read as absolute.
#[test]
fn editing_a_fence_delimiter_rescans_everything() {
    let text = "```\ncode\n```\nafter";
    let (text, edit) = apply(text, 3..3, "`");
    assert_eq!(dirty_range(&text, &edit),
               0..text.len(),
               "a delimiter that changed length can have opened or closed the block, and every \
                line after it reads differently");
}
