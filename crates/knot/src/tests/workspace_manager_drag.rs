//! Reordering the Workspace Manager's rows by dragging one by its handle.
//!
//! The preview and the drop line are layout, so those are asked of a drawn
//! window with a real drag driven through it: the handle pressed, the
//! mouse moved past gpui's drag threshold, then released over a row.

use gpui_kit::Bounds;
use gpui_kit::Modifiers;
use gpui_kit::MouseButton;
use gpui_kit::Pixels;
use gpui_kit::Point;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;
use uuid::Uuid;

use crate::tests::workspace;
use crate::tests::workspace_dialog::manager;
use crate::workspace_manager::DropEdge;
use crate::workspace_manager::drop_gap;
use crate::workspace_manager::drop_line_edge;

const WINDOW: (f32, f32) = (600., 400.);

/// A manager showing `first`, `second`, `third`, in a window that fits them.
fn three_rows(
    cx: &mut TestAppContext)
    -> (VisualTestContext, [Uuid; 3], std::sync::Arc<parking_lot::Mutex<knot_agents::AgentStore>>) {
    let (store, cx, _manager) = manager(cx);
    let ids = ["first", "second", "third"].map(|name| {
                                              let workspace = workspace(name);
                                              let id = workspace.id;
                                              store.lock().add_workspace(workspace);
                                              id
                                          });
    cx.simulate_resize(size(px(WINDOW.0), px(WINDOW.1)));
    cx.run_until_parked();
    (cx, ids, store)
}

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(selector)
      .unwrap_or_else(|| panic!("{selector} was never painted"))
}

/// Presses the first row's handle and drags it to `to`, without releasing.
/// Returns where the drag started, which is the first move past gpui's 2px
/// threshold - not the press - and is what the preview is anchored to.
fn drag_first_row_to(cx: &mut VisualTestContext, to: Point<Pixels>) -> Point<Pixels> {
    let handle = bounds(cx, "workspace-drag-handle-0").center();
    let start = handle + point(px(0.), px(10.));
    cx.simulate_mouse_down(handle, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    start
}

fn lower_half(row: Bounds<Pixels>) -> Point<Pixels> {
    point(row.center().x, row.origin.y + row.size.height * 0.75)
}

#[test]
fn a_gap_beside_the_dragged_row_is_no_drop_at_all() {
    assert_eq!(drop_gap(1, DropEdge::Top, 1), None, "its own top edge");
    assert_eq!(drop_gap(1, DropEdge::Bottom, 1),
               None,
               "its own bottom edge");
    assert_eq!(drop_gap(0, DropEdge::Bottom, 1),
               None,
               "the edge it shares with the row above");
    assert_eq!(drop_gap(2, DropEdge::Top, 1),
               None,
               "the edge it shares with the row below");
    assert_eq!(drop_gap(0, DropEdge::Top, 1), Some(0));
    assert_eq!(drop_gap(2, DropEdge::Bottom, 1), Some(3));
}

#[test]
fn each_gap_is_drawn_once_on_the_row_below_it_or_under_the_last() {
    let rows = 3;
    let drawn = |gap| {
        (0..rows).filter_map(|row| drop_line_edge(row, rows, gap).map(|edge| (row, edge)))
                 .collect::<Vec<_>>()
    };
    assert_eq!(drawn(0), vec![(0, DropEdge::Top)]);
    assert_eq!(drawn(2), vec![(2, DropEdge::Top)]);
    assert_eq!(drawn(3), vec![(2, DropEdge::Bottom)]);
}

#[gpui_kit::test]
fn the_drag_preview_is_laid_over_the_row_it_was_lifted_from(cx: &mut TestAppContext) {
    let (mut cx, _ids, _store) = three_rows(cx);
    let row = bounds(&mut cx, "workspace-row-0");
    let to = bounds(&mut cx, "workspace-drag-handle-0").center() + point(px(0.), px(30.));
    let moved = to - drag_first_row_to(&mut cx, to);

    let preview = bounds(&mut cx, "workspace-drag-preview");
    assert_eq!(preview.size, row.size, "the preview is the row's size");
    assert_eq!(preview.origin,
               row.origin + moved,
               "the preview sits where the row is, moved with the cursor");
    assert!(preview.origin.x >= px(0.) && preview.right() <= px(WINDOW.0),
            "the preview fits across the window rather than starting at the handle: {preview:?}");
}

#[gpui_kit::test]
fn dropping_on_the_lower_half_of_a_row_moves_the_dragged_row_below_it(cx: &mut TestAppContext) {
    let (mut cx, [first, second, third], store) = three_rows(cx);
    let target = lower_half(bounds(&mut cx, "workspace-row-1"));
    drag_first_row_to(&mut cx, target);

    let line = bounds(&mut cx, "workspace-drop-line");
    let row_two = bounds(&mut cx, "workspace-row-2");
    assert!(line.bottom() <= row_two.top()
            && line.top() >= bounds(&mut cx, "workspace-row-1").bottom(),
            "the line is in the gap between the second and third rows: {line:?}");

    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    let order: Vec<Uuid> = store.lock()
                                .workspaces()
                                .iter()
                                .map(|workspace| workspace.id)
                                .collect();
    assert_eq!(order, vec![second, first, third]);
    assert!(cx.debug_bounds("workspace-drop-line").is_none(),
            "the line goes when the drag ends");
}

#[gpui_kit::test]
fn dropping_below_the_last_row_moves_the_dragged_row_to_the_end(cx: &mut TestAppContext) {
    let (mut cx, [first, second, third], store) = three_rows(cx);
    let target = lower_half(bounds(&mut cx, "workspace-row-2"));
    drag_first_row_to(&mut cx, target);
    assert!(bounds(&mut cx, "workspace-drop-line").top()
            >= bounds(&mut cx, "workspace-row-2").bottom(),
            "the line is under the last row");

    cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    let order: Vec<Uuid> = store.lock()
                                .workspaces()
                                .iter()
                                .map(|workspace| workspace.id)
                                .collect();
    assert_eq!(order, vec![second, third, first]);
}

#[gpui_kit::test]
fn a_drag_over_its_own_row_shows_no_line(cx: &mut TestAppContext) {
    let (mut cx, _ids, _store) = three_rows(cx);
    let own = bounds(&mut cx, "workspace-row-0").center();
    drag_first_row_to(&mut cx, own);
    assert!(cx.debug_bounds("workspace-drop-line").is_none(),
            "a drop here would change nothing, so nothing is offered");
}
