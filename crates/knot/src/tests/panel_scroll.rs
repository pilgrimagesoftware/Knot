//! When the agent conversation offers to jump back to its latest row
//! (`openspec/specs/acp-panel-ui`, the "scroll to latest" control).
//!
//! These need a real window: the question is about layout, and the whole
//! defect was reading a quantity a virtualized list does not have. The probe
//! below is the panel's list reduced to what matters - a `ListState` with
//! `LIST_OVERDRAW`, fixed-height rows, and a viewport smaller than the
//! content - so a failure here is about the predicate, not about markdown.

use gpui_kit::AppContext;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ListAlignment;
use gpui_kit::ListOffset;
use gpui_kit::ListState;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowBounds;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;

use crate::panel_view;

/// Every row the same height, so what the assertions mean is arithmetic
/// rather than a guess about how tall a rendered message came out.
const ROW_HEIGHT: f32 = 100.;

/// Tall enough to hold four rows, short enough that 40 of them do not fit -
/// the shape the control exists for.
const VIEWPORT_HEIGHT: f32 = 400.;

const ROW_COUNT: usize = 40;

/// The panel's list, with its rows replaced by blank ones of a known height.
struct ListProbe {
    list: ListState,
}

impl Render for ListProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
             .child(gpui_kit::list(self.list.clone(), |_index, _window, _cx| {
                        div().w_full().h(px(ROW_HEIGHT)).into_any_element()
                    }).size_full())
    }
}

/// A drawn window holding `rows` rows, and the list backing them.
fn probe(cx: &mut TestAppContext, rows: usize)
         -> (VisualTestContext, Entity<ListProbe>, ListState) {
    let list = ListState::new(rows, ListAlignment::Top, px(panel_view::LIST_OVERDRAW));
    let mut probe = None;
    let window = {
        let probe = &mut probe;
        let list = list.clone();
        cx.update(|cx| {
              gpui_kit::init(cx);
              let bounds = Bounds { origin: point(px(0.), px(0.)),
                                    size:   size(px(600.), px(VIEWPORT_HEIGHT)), };
              cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)),
                                             ..WindowOptions::default() },
                             |window, cx| {
                                 let view = cx.new(|_| ListProbe { list });
                                 *probe = Some(view.clone());
                                 cx.new(|cx| Root::new(view, window, cx))
                             })
                .expect("the probe window should open")
          })
    };
    let probe_cx = VisualTestContext::from_window(window.into(), cx);
    probe_cx.run_until_parked();
    (probe_cx, probe.expect("the probe was built"), list)
}

/// Redraws, so the list lays out from whatever scroll position was just set.
fn redraw(probe_cx: &mut VisualTestContext, probe: &Entity<ListProbe>) {
    probe_cx.update(|_window, cx| probe.update(cx, |_, cx| cx.notify()));
    probe_cx.run_until_parked();
}

/// The control has nothing to offer when the latest row is already on
/// screen. Without this the rest is satisfied by a predicate that just
/// answers `true`.
#[gpui_kit::test]
fn a_conversation_at_its_latest_row_offers_no_jump(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, list) = probe(cx, ROW_COUNT);

    list.scroll_to_end();
    redraw(&mut probe_cx, &probe);

    assert!(!panel_view::scrolled_away_from_tail(&list, ROW_COUNT),
            "the list is parked at its last row, so there is nowhere to jump to");
}

#[gpui_kit::test]
fn a_conversation_scrolled_to_its_top_offers_the_jump(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, list) = probe(cx, ROW_COUNT);

    list.scroll_to(ListOffset { item_ix:        0,
                                offset_in_item: px(0.), });
    redraw(&mut probe_cx, &probe);

    assert!(panel_view::scrolled_away_from_tail(&list, ROW_COUNT),
            "36 rows below the viewport is exactly what the control is for");
}

/// The reported regression. The user reads back through a conversation while
/// the agent is still writing; each new row arrives as an unmeasured item at
/// the tail, and the control has to keep offering the way back.
#[gpui_kit::test]
fn rows_arriving_while_the_user_reads_further_up_keep_the_jump_offered(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, list) = probe(cx, ROW_COUNT);

    list.scroll_to(ListOffset { item_ix:        0,
                                offset_in_item: px(0.), });
    redraw(&mut probe_cx, &probe);

    list.splice(ROW_COUNT..ROW_COUNT, 1);
    redraw(&mut probe_cx, &probe);

    assert!(panel_view::scrolled_away_from_tail(&list, ROW_COUNT + 1),
            "a row appended out of view is the whole reason to offer the jump");
}

/// What the predicate used to be built on, kept as a probe of the
/// dependency rather than of our code: `is_scrolled_to_end` needs the total
/// content height, a virtualized list does not have one, and so it answers
/// `None` in precisely the situation above. If a future `gpui-pre` ever
/// makes it answer `Some(false)` here, this is the test that says the
/// simpler predicate has become available again.
#[gpui_kit::test]
fn the_lists_own_end_check_cannot_answer_while_rows_are_unmeasured(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, list) = probe(cx, ROW_COUNT);

    list.scroll_to(ListOffset { item_ix:        0,
                                offset_in_item: px(0.), });
    redraw(&mut probe_cx, &probe);

    assert_eq!(list.is_scrolled_to_end(),
               None,
               "rows outside the viewport are unmeasured, so the list cannot say how tall it is - \
                which is why the control went missing");
}

/// A conversation shorter than its pane is not scrollable, so the control
/// stays hidden rather than offering a jump to where the view already is.
#[gpui_kit::test]
fn a_conversation_shorter_than_its_pane_offers_no_jump(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, list) = probe(cx, 2);

    redraw(&mut probe_cx, &probe);

    assert!(!panel_view::scrolled_away_from_tail(&list, 2));
}

#[gpui_kit::test]
fn an_empty_conversation_offers_no_jump(cx: &mut TestAppContext) {
    let (mut probe_cx, probe, list) = probe(cx, 0);

    redraw(&mut probe_cx, &probe);

    assert!(!panel_view::scrolled_away_from_tail(&list, 0));
}
