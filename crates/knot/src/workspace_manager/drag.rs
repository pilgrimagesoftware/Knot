//! Reordering the roster by dragging a row by its handle.
//!
//! Three things a drag has to show, all of which used to be missing: which
//! row is moving (the preview, a copy of the row laid over it), where it
//! would land (a line in the gap under the cursor), and nothing at all
//! where a drop would change nothing. The shape follows gpui-kit's own
//! reorderable list story: each row decides from the cursor's half which
//! gap it is offering, and the gap - not the row - is what is drawn and
//! what the store is asked to move into.
//!
//! Not here: what a row looks like (`render`), and the store's reorder
//! itself (`AgentStore::move_workspace_to_gap`).

use gpui_kit::App;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::DragMoveEvent;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Point;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use super::render::drag_handle;
use super::render::row_summary;
use super::state::DropTarget;
use crate::workspace_manager::WorkspaceManager;

/// What a row carries while it is dragged.
#[derive(Clone)]
pub(crate) struct WorkspaceDrag {
    pub(crate) id:  Uuid,
    /// Its place in the order the drag started from, which is the order
    /// every gap is counted in.
    pub(crate) row: usize,
}

/// Which edge of a row the cursor is nearer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DropEdge {
    Top,
    Bottom,
}

/// The gap a drop over `edge` of `row` would move the dragged row `from`
/// into, or `None` when that gap is on either side of it and the drop
/// would leave the order as it is.
pub(crate) fn drop_gap(row: usize, edge: DropEdge, from: usize) -> Option<usize> {
    let gap = match edge {
        DropEdge::Top => row,
        DropEdge::Bottom => row + 1,
    };
    (gap != from && gap != from + 1).then_some(gap)
}

/// The line a row draws for a gap, if the gap is one of its edges.
///
/// Every gap but the last is drawn as the top edge of the row below it, so
/// the bottom half of one row and the top half of the next light the same
/// line; only the last row draws a bottom edge, for the gap below it.
pub(crate) fn drop_line_edge(row: usize, rows: usize, gap: usize) -> Option<DropEdge> {
    if gap == row {
        Some(DropEdge::Top)
    }
    else if gap == rows && row + 1 == rows {
        Some(DropEdge::Bottom)
    }
    else {
        None
    }
}

/// The insertion line, set in the gap outside `edge` of a row that is
/// `relative()`. 2px in the middle of the rows' 8px gap.
pub(crate) fn drop_line(edge: DropEdge, cx: &App) -> impl IntoElement + use<> {
    let line = div().debug_selector(|| "workspace-drop-line".into())
                    .absolute()
                    .left_0()
                    .right_0()
                    .h(px(2.))
                    .rounded_full()
                    .bg(cx.theme().drag_border);
    match edge {
        DropEdge::Top => line.top(px(-5.)),
        DropEdge::Bottom => line.bottom(px(-5.)),
    }
}

impl WorkspaceManager {
    /// A drag moved somewhere in the window, as heard by row `row`: offer
    /// the gap at the cursor's edge when the cursor is over this row, and
    /// withdraw this row's offer when it is not.
    pub(super) fn drag_moved(&mut self, row: usize, event: &DragMoveEvent<WorkspaceDrag>,
                             cx: &mut Context<Self>) {
        let position = event.event.position;
        let from = event.drag(cx).row;
        let gap = event.bounds
                       .contains(&position)
                       .then(|| {
                           if position.y < event.bounds.center().y {
                               DropEdge::Top
                           }
                           else {
                               DropEdge::Bottom
                           }
                       })
                       .and_then(|edge| drop_gap(row, edge, from));
        let target = match gap {
            Some(gap) => Some(DropTarget { row, gap }),
            // Only this row's own offer is withdrawn: the row the cursor
            // moved onto may already have made one.
            None if self.drag.target.is_some_and(|target| target.row == row) => None,
            None => return,
        };
        if self.drag.target != target {
            self.drag.target = target;
            cx.notify();
        }
    }

    /// The drag was released over row `row`: move into the gap it offered.
    pub(super) fn dropped(&mut self, drag: &WorkspaceDrag, row: usize, cx: &mut Context<Self>) {
        let target = self.drag.target.take();
        cx.notify();
        let Some(target) = target.filter(|target| target.row == row)
        else {
            return;
        };
        if self.store.lock().move_workspace_to_gap(drag.id, target.gap) {
            self.persist(cx);
        }
    }
}

/// What follows the cursor during a drag: a copy of the row it was lifted
/// from, in the foreground colour - the preview is drawn outside the root
/// view, so it inherits no text colour and was black on a dark theme.
pub(crate) struct WorkspaceDragPreview {
    pub(crate) name:   String,
    pub(crate) agents: String,
    /// The lifted row's bounds relative to the handle the drag started on.
    /// gpui puts the preview's origin at the handle, which sits at the
    /// row's right end, so without this offset the copy starts at the
    /// handle and runs off the window. `None` if the row was never painted.
    pub(crate) row:    Option<Bounds<Pixels>>,
}

impl WorkspaceDragPreview {
    /// `row` in window coordinates, `handle` the dragged handle's origin.
    pub(crate) fn new(name: String, agents: String, row: Option<Bounds<Pixels>>,
                      handle: Point<Pixels>)
                      -> Self {
        let row = row.map(|row| Bounds { origin: row.origin - handle,
                                         size:   row.size, });
        Self { name, agents, row }
    }
}

impl Render for WorkspaceDragPreview {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card = h_flex().debug_selector(|| "workspace-drag-preview".into())
                           .items_center()
                           .gap_3()
                           .p_3()
                           .rounded(cx.theme().radius)
                           .border_1()
                           .border_color(cx.theme().drag_border)
                           .bg(cx.theme().background)
                           .shadow_lg()
                           .opacity(0.9)
                           .text_color(cx.theme().foreground)
                           .child(row_summary(self.name.clone(), self.agents.clone(), cx))
                           .child(drag_handle());
        // Positioned inside an empty root rather than by margins on the card:
        // gpui lays the drag view out as a root at the handle's origin, and
        // a root's margins do not move it.
        let card = match self.row {
            Some(row) => card.absolute()
                             .left(row.origin.x)
                             .top(row.origin.y)
                             .w(row.size.width)
                             .h(row.size.height),
            None => card.w(px(320.)),
        };
        div().child(card)
    }
}
