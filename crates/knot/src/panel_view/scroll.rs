//! Whether the conversation is scrolled away from its latest row - the one
//! question the "scroll to latest" control's visibility turns on.
//!
//! This looks harder than it is because the list is virtualized. A
//! `ListState` only measures the rows intersecting the viewport plus
//! [`super::LIST_OVERDRAW`], so the *total* content height is unknown for
//! any conversation longer than a screen, and every answer that needs it -
//! `ListState::is_scrolled_to_end`, the scrollbar's maximum offset - comes
//! back `None` or approximate. Asking instead where the last row sits needs
//! only the rows the list has already laid out.

use gpui_kit::ListState;
use gpui_kit::px;

/// How far past the viewport's bottom edge the last row may still sit and
/// count as "at the bottom".
///
/// Sub-pixel layout noise otherwise toggles the control between frames.
const TAIL_EPSILON: f32 = 8.;

/// Whether the conversation's last row is out of view, so the "scroll to
/// latest" control has somewhere to jump to.
///
/// `row_count` is the list's item count for this frame, as reconciled by
/// [`super::sync_row_count`].
///
/// False before the list's first layout and for a conversation that fits its
/// viewport: there is no tail to return to in either case.
pub(crate) fn scrolled_away_from_tail(list: &ListState, row_count: usize) -> bool {
    let viewport = list.viewport_bounds();
    // Zero height is either a list that has not been laid out yet or a pane
    // squeezed to nothing. Neither is a scroll position.
    if viewport.size.height <= px(0.) {
        return false;
    }
    let Some(last) = row_count.checked_sub(1)
    else {
        return false;
    };
    // `scroll_to_end` and tail-following park the anchor one past the last
    // row, and the next layout walks back from there to a real one. Reading
    // that transient as "scrolled up" flashes the control for a frame at
    // the exact moment the user pressed it.
    if list.logical_scroll_top().item_ix > last {
        return false;
    }
    match list.bounds_for_item(last) {
        // The last row has never been laid out, which for a list that
        // measures from the scroll position downwards means it is below the
        // viewport. This is the case the control exists for: rows arriving
        // from the agent while the user reads further up.
        None => true,
        Some(bounds) => bounds.bottom() > viewport.bottom() + px(TAIL_EPSILON),
    }
}
