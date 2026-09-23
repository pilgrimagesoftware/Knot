//! The virtualized list's row model.
//!
//! The conversation is one `ListState` whose item count has to track the
//! panel state's messages plus its trailing rows (a pending permission
//! request, a session-end banner). [`sync_row_count`] splices only what
//! changed, so off-screen rows keep their measured heights - which is the
//! whole point of virtualizing a conversation that can run to thousands of
//! tool calls.

use gpui_kit::ListState;

use crate::panel_state::PanelState;

/// One virtualized row of the conversation: every message in order, then
/// the pending permission prompt, then the ended-session banner. Modelling
/// the trailing cards as rows of the same list (rather than siblings below
/// a scroller) keeps them inside the virtualizer and lets tail-following
/// track them like any other row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelRow {
    /// `PanelState::messages[index]`.
    Message(usize),
    /// The pending permission prompt, when one is awaiting a decision.
    Permission,
    /// The ended-session banner, when the session has ended.
    Ended,
    /// The working indicator while the turn is active.
    Working,
}

/// The number of rows `state` needs the conversation list to lay out -
/// every message, plus a permission row and/or an ended row when present.
pub(crate) fn row_count(state: &PanelState) -> usize {
    state.messages.len()
    + usize::from(state.pending_permission.is_some())
    + usize::from(state.ended.is_some())
    + usize::from(state.turn_active)
}

/// Resolves a list index to the row it draws. `None` for an index past the
/// end, which the list can briefly hold between a row landing in
/// `PanelState` and the next `sync_row_count` splice.
pub(crate) fn row_at(state: &PanelState, index: usize) -> Option<PanelRow> {
    let messages = state.messages.len();
    let has_permission = state.pending_permission.is_some();
    if index < messages {
        Some(PanelRow::Message(index))
    }
    else if index == messages && has_permission {
        Some(PanelRow::Permission)
    }
    else if index == messages + usize::from(has_permission) && state.ended.is_some() {
        Some(PanelRow::Ended)
    }
    else if index == messages + usize::from(has_permission) + usize::from(state.ended.is_some())
              && state.turn_active
    {
        Some(PanelRow::Working)
    }
    else {
        None
    }
}

/// Brings the list's item count in step with `state`'s rows, splicing only
/// the delta so off-screen rows keep the heights they were already
/// measured at and the reader's scroll position is left alone. `known` is
/// the count the list currently holds, and the returned count should be
/// passed back as `known` next frame.
pub(crate) fn sync_row_count(list: &ListState, known: usize, state: &PanelState) -> usize {
    let count = row_count(state);
    if count > known {
        list.splice(known..known, count - known);
    }
    else if count < known {
        list.splice(count..known, 0);
    }
    count
}
