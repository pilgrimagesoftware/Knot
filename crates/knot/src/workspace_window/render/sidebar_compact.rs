//! The compact agent row: what a sidebar row shows once it is too narrow for
//! the full one.
//!
//! Contract: `openspec/specs/agent-list-ui/spec.md` - "A narrow sidebar shows
//! a compact layout". Ported from `AgentRowView.compactBody` in the Swift
//! reference.
//!
//! Only the row's *body* lives here. The row's frame - its id, selection
//! background, the dimming of a stopped agent, the companion indent, the
//! click handler and the context menu - stays in [`super::sidebar`] and is
//! identical either way, which is the point: two full row layouts would drift
//! apart the first time one of those changed.

use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::div;
use gpui_kit::px;

use crate::app_state::state_color;

/// What the compact row draws. A struct rather than three positional
/// parameters, and deliberately a subset of [`AgentRow`]: everything the
/// compact layout hides is absent here rather than passed and ignored. The
/// name is not among these - it becomes the row frame's tooltip, which
/// [`super::sidebar`] owns.
pub(crate) struct CompactAgentRow {
    /// Already clamped to a single grapheme by the caller.
    pub avatar:   String,
    pub state:    knot_agents::AgentState,
    pub is_shell: bool,
}

/// The compact row's body: the avatar alone, centred, with the state dot
/// overlaid on its bottom-trailing corner.
///
/// The dot moves onto the avatar because the text block it used to sit
/// beside is gone; it keeps its colour mapping and its "a shell agent has
/// none" rule, so where it sits is the only thing that changes.
pub(crate) fn compact_agent_row_body(row: CompactAgentRow) -> impl IntoElement {
    let CompactAgentRow { avatar,
                          state,
                          is_shell, } = row;
    let dot = (!is_shell).then(|| {
                             div().absolute()
                                  .bottom_0()
                                  .right_0()
                                  .w(px(8.))
                                  .h(px(8.))
                                  .rounded_full()
                                  .bg(state_color(state))
                         });
    let tile = div().relative()
                    .w(px(40.))
                    .h(px(40.))
                    .flex_shrink_0()
                    // As on the full-width row: a legacy avatar carrying more
                    // than one glyph is clamped by the caller, and this keeps
                    // an oversized one inside the tile either way. The dot
                    // sits within these bounds, so it is not what gets cut.
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_2xl()
                    .child(avatar)
                    .children(dot);
    h_flex().w_full().justify_center().child(tile)
}
