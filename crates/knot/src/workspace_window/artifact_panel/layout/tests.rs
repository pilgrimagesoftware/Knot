//! Ports `SkwadTests/Views/Artifacts/ArtifactPanelLayoutTests.swift`, whose
//! four cases and sum invariant are what this arithmetic has to hold.

use super::{clamp_split, clamp_width, section_heights};
use crate::consts;

/// The section area the Swift suite uses, so the cases below are comparable
/// to it line for line.
const TOTAL: f32 = 600.;

#[test]
fn both_expanded_at_the_default_split_divides_evenly() {
    let (markdown, mermaid) =
        section_heights(TOTAL, consts::ARTIFACT_PANEL_DEFAULT_SPLIT, false, false);
    let available = TOTAL - consts::ARTIFACT_PANEL_DIVIDER_HEIGHT;

    assert_eq!(markdown, available / 2.);
    assert_eq!(mermaid, available / 2.);
}

#[test]
fn both_expanded_follows_the_split_ratio() {
    let (markdown, mermaid) = section_heights(TOTAL, 0.7, false, false);

    assert!(markdown > mermaid,
            "a 0.7 split should give the markdown section the larger share: {markdown} vs \
             {mermaid}");
}

/// The Swift suite's `bothExpandedSumToTotal`, over the same five ratios. The
/// sum is the property a reader sees broken - a gap below the lower section,
/// or a section clipped by the panel's edge.
#[test]
fn the_two_sections_and_the_divider_fill_the_section_area() {
    for ratio in [0.15, 0.3, 0.5, 0.7, 0.85] {
        let (markdown, mermaid) = section_heights(TOTAL, ratio, false, false);
        let sum = markdown + mermaid + consts::ARTIFACT_PANEL_DIVIDER_HEIGHT;

        assert!((sum - TOTAL).abs() < 0.01,
                "at ratio {ratio} the sections and divider came to {sum}, not {TOTAL}");
    }
}

#[test]
fn a_collapsed_markdown_section_is_its_header_and_the_other_takes_the_rest() {
    let (markdown, mermaid) = section_heights(TOTAL, 0.5, true, false);

    assert_eq!(markdown, consts::ARTIFACT_SECTION_HEADER_HEIGHT);
    assert_eq!(mermaid, TOTAL - consts::ARTIFACT_SECTION_HEADER_HEIGHT);
}

#[test]
fn a_collapsed_mermaid_section_is_its_header_and_the_other_takes_the_rest() {
    let (markdown, mermaid) = section_heights(TOTAL, 0.5, false, true);

    assert_eq!(markdown, TOTAL - consts::ARTIFACT_SECTION_HEADER_HEIGHT);
    assert_eq!(mermaid, consts::ARTIFACT_SECTION_HEADER_HEIGHT);
}

/// Both collapsed shows two headers and nothing else - neither section takes
/// the remainder, because there is no section left to give it to.
#[test]
fn both_collapsed_are_two_headers() {
    let (markdown, mermaid) = section_heights(TOTAL, 0.5, true, true);

    assert_eq!(markdown, consts::ARTIFACT_SECTION_HEADER_HEIGHT);
    assert_eq!(mermaid, consts::ARTIFACT_SECTION_HEADER_HEIGHT);
}

/// A collapsed section does not take the split with it: expanding again
/// returns to the ratio the user had set, rather than to the default.
#[test]
fn collapsing_does_not_disturb_the_split() {
    let before = section_heights(TOTAL, 0.7, false, false);
    let _collapsed = section_heights(TOTAL, 0.7, true, false);
    let after = section_heights(TOTAL, 0.7, false, false);

    assert_eq!(before, after);
}

#[test]
fn a_split_outside_the_bounds_is_clamped_at_both_ends() {
    assert_eq!(clamp_split(0.), consts::ARTIFACT_PANEL_MIN_SPLIT);
    assert_eq!(clamp_split(1.), consts::ARTIFACT_PANEL_MAX_SPLIT);
    assert_eq!(clamp_split(0.5),
               0.5,
               "a ratio inside the bounds is left alone");
}

/// A ratio past the clamp cannot produce a negative height, which is what the
/// clamp inside `section_heights` is there to guarantee.
#[test]
fn an_out_of_bounds_split_still_yields_two_positive_heights() {
    for ratio in [-5., 0., 1., 42.] {
        let (markdown, mermaid) = section_heights(TOTAL, ratio, false, false);

        assert!(markdown > 0.,
                "markdown height was {markdown} at ratio {ratio}");
        assert!(mermaid > 0.,
                "mermaid height was {mermaid} at ratio {ratio}");
    }
}

#[test]
fn a_width_outside_the_bounds_is_clamped_at_both_ends() {
    assert_eq!(clamp_width(0.), consts::ARTIFACT_PANEL_MIN_WIDTH);
    assert_eq!(clamp_width(5000.), consts::ARTIFACT_PANEL_MAX_WIDTH);
    assert_eq!(clamp_width(consts::ARTIFACT_PANEL_DEFAULT_WIDTH),
               consts::ARTIFACT_PANEL_DEFAULT_WIDTH);
}
