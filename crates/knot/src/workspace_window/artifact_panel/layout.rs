//! How the artifact panel's section area divides between its two sections
//! (`openspec/specs/artifact-panel`, "A divider splits the two sections and
//! can be dragged" and "Either section can be collapsed to its header").
//!
//! Ported from `ArtifactPanelView.sectionHeight`, which is a static function
//! in Swift for the same reason it is a free function here: the invariant
//! worth testing is arithmetic, and a test for it should not need a window.
//!
//! Flex weights cannot express this. A collapsed section is a fixed header
//! height with the other section taking the remainder, which is a different
//! rule from a ratio - encoding both in flex means a weight whose meaning
//! changes with the collapse flags.

#[cfg(test)]
mod tests;

use crate::consts;

/// The heights of the markdown and mermaid sections, in that order.
///
/// `total` is the panel's *section area* - what is left below the toolbar,
/// which sits above both sections and is not divided by the split.
///
/// Both are returned together rather than answered one at a time because the
/// invariant worth holding is about the pair: the two heights plus the
/// divider fill the section area exactly, at every split.
///
/// The divider is counted out only when it is drawn, which is when both
/// sections are expanded. A collapsed section means no split left to set, so
/// no divider - see the spec's "No divider while collapsed".
pub(in crate::workspace_window) fn section_heights(total: f32, split_ratio: f32,
                                                   markdown_collapsed: bool,
                                                   mermaid_collapsed: bool)
                                                   -> (f32, f32) {
    let header = consts::ARTIFACT_SECTION_HEADER_HEIGHT;

    match (markdown_collapsed, mermaid_collapsed) {
        // Two headers and nothing else. Neither takes the remainder: there is
        // no section left to give it to.
        (true, true) => (header, header),
        (true, false) => (header, total - header),
        (false, true) => (total - header, header),
        (false, false) => {
            let available = total - consts::ARTIFACT_PANEL_DIVIDER_HEIGHT;
            let split = clamp_split(split_ratio);
            let markdown = available * split;
            // The remainder rather than `available * (1 - split)`: two
            // multiplications of the same float do not necessarily sum back
            // to it, and the sum is the property the tests pin.
            (markdown, available - markdown)
        }
    }
}

/// `ratio` held inside the bounds a divider drag may reach.
///
/// Applied here rather than only where the drag is read, so the arithmetic
/// above cannot be handed a ratio it would turn into a negative height.
pub(in crate::workspace_window) fn clamp_split(ratio: f32) -> f32 {
    ratio.clamp(consts::ARTIFACT_PANEL_MIN_SPLIT,
                consts::ARTIFACT_PANEL_MAX_SPLIT)
}

/// `width` held inside the bounds a panel-edge drag may reach.
pub(in crate::workspace_window) fn clamp_width(width: f32) -> f32 {
    width.clamp(consts::ARTIFACT_PANEL_MIN_WIDTH,
                consts::ARTIFACT_PANEL_MAX_WIDTH)
}
