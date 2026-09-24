//! The arrangement's own rules: what a `display-markdown` call moves, and
//! what closing a section does and does not disturb.
//!
//! [`ArtifactPanelArrangement::reseed_expanded`] is the subject. It is where
//! the three readings of "`maximized` takes effect" had to be settled into
//! one, so the cases that separate them are the cases worth pinning.

use std::path::PathBuf;

use super::ArtifactPanelArrangement;

fn file(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/{name}.md"))
}

#[test]
fn a_fresh_arrangement_is_unexpanded_at_the_defaults() {
    let arrangement = ArtifactPanelArrangement::default();

    assert!(!arrangement.expanded);
    assert!(!arrangement.markdown_collapsed);
    assert!(!arrangement.mermaid_collapsed);
    assert_eq!(arrangement.width,
               crate::consts::ARTIFACT_PANEL_DEFAULT_WIDTH);
    assert_eq!(arrangement.split,
               crate::consts::ARTIFACT_PANEL_DEFAULT_SPLIT);
}

#[test]
fn a_maximized_file_expands_the_panel() {
    let mut arrangement = ArtifactPanelArrangement::default();

    let moved = arrangement.reseed_expanded(Some(&file("a")), true);

    assert!(moved, "the panel moved from unexpanded to expanded");
    assert!(arrangement.expanded);
}

/// The case the file-alone trigger strands: the panel is already open on this
/// file, and the agent shows it again asking for it maximized.
#[test]
fn re_showing_the_open_file_maximized_still_expands() {
    let mut arrangement = ArtifactPanelArrangement::default();
    arrangement.reseed_expanded(Some(&file("a")), false);

    let moved = arrangement.reseed_expanded(Some(&file("a")), true);

    assert!(moved,
            "the same file with a different argument says something new");
    assert!(arrangement.expanded);
}

/// The case an every-call trigger breaks: the user expanded the panel by
/// hand, and the agent re-shows the file it has just edited, with `maximized`
/// omitted as it always was.
#[test]
fn re_showing_the_same_file_unchanged_leaves_a_hand_expanded_panel() {
    let mut arrangement = ArtifactPanelArrangement::default();
    arrangement.reseed_expanded(Some(&file("a")), false);
    arrangement.expanded = true;

    let moved = arrangement.reseed_expanded(Some(&file("a")), false);

    assert!(!moved,
            "nothing about the call changed, so nothing should move");
    assert!(arrangement.expanded,
            "the user's own toggle survives an agent repeating itself");
}

#[test]
fn a_second_file_without_the_argument_returns_the_panel_to_its_width() {
    let mut arrangement = ArtifactPanelArrangement::default();
    arrangement.reseed_expanded(Some(&file("a")), true);

    let moved = arrangement.reseed_expanded(Some(&file("b")), false);

    assert!(moved);
    assert!(!arrangement.expanded);
}

/// `clear_markdown_panel` sets `markdown_maximized` false as it clears the
/// file. A re-seed that followed that would collapse a panel whose diagram is
/// still on screen, so a call with no file writes nothing.
#[test]
fn closing_the_markdown_section_does_not_collapse_an_expanded_panel() {
    let mut arrangement = ArtifactPanelArrangement::default();
    arrangement.reseed_expanded(Some(&file("a")), true);

    let moved = arrangement.reseed_expanded(None, false);

    assert!(!moved,
            "clearing the file says nothing about the panel's size");
    assert!(arrangement.expanded,
            "the diagram is still open, so the panel stays expanded");
}
