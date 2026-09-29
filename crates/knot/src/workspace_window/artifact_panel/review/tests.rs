//! The review flow's pure parts: where it starts, the reply it begins, and
//! the font size steps' bounds.

use super::ReviewState;
use super::review_preamble;
use super::stepped_font_size;
use crate::consts;

#[test]
fn a_file_starts_out_being_read() {
    assert_eq!(ReviewState::default(), ReviewState::Viewing);
}

/// The agent may have several artifacts in flight; the reply has to say
/// which one the comments are about.
#[test]
fn the_review_preamble_names_the_file_and_leaves_room_for_comments() {
    let preamble = review_preamble("plan.md");

    assert!(preamble.contains("plan.md"), "{preamble}");
    assert!(preamble.ends_with('\n'),
            "the comments go on the line after the preamble");
}

#[test]
fn the_font_size_steps_within_its_bounds() {
    assert_eq!(stepped_font_size(14, 1), Some(15));
    assert_eq!(stepped_font_size(14, -1), Some(13));
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MAX, 1), None);
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MIN, -1), None);
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MAX, -1),
               Some(consts::MARKDOWN_FONT_SIZE_MAX - 1));
}

/// A stored size outside the bounds - a hand-edited settings file - still
/// steps back toward them rather than being stuck.
#[test]
fn an_out_of_range_size_can_step_back_in() {
    assert_eq!(stepped_font_size(consts::MARKDOWN_FONT_SIZE_MAX + 1, -1),
               Some(consts::MARKDOWN_FONT_SIZE_MAX));
}
