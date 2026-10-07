//! Scroll regions show a scrollbar while they scroll (issue #583).
//!
//! Knot's scroll regions come in two layouts, and each takes its scrollbar a
//! different way: a region that fills its column uses gpui-component's
//! `overflow_y_scrollbar`, and a capped one uses [`capped_scroll`], because
//! that wrapper stretched a short capped list to its full cap and stopped an
//! overflowing one from scrolling. Getting either wrong leaves a list that
//! grows past its space or no longer scrolls, which no build catches. These
//! probes reproduce the two layouts with rows of a known height, so the
//! assertions are arithmetic.

use gpui_kit::AppContext;
use gpui_kit::Bounds;
use gpui_kit::Context;
use gpui_kit::Div;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ListAlignment;
use gpui_kit::ListState;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::ScrollDelta;
use gpui_kit::ScrollWheelEvent;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowBounds;
use gpui_kit::WindowOptions;
use gpui_kit::base::v_flex;
use gpui_kit::component::Root;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;

use crate::capped_scroll::capped_scroll;

const WINDOW_HEIGHT: f32 = 400.;
const ROW_HEIGHT: f32 = 50.;
/// The cap a bounded list scrolls inside, as the Personas, Prompts and Bench
/// lists, the bench popover and the Processes and MCP bodies do.
const CAP: f32 = 120.;
const BAR_HEIGHT: f32 = 20.;

/// The two ways Knot sizes a scroll region.
#[derive(Clone, Copy)]
enum Shape {
    /// `max_h` on the region itself, with something after it that must sit
    /// directly below the cap.
    Capped,
    /// `flex_1().min_h_0()` between a header and a footer in a fixed-height
    /// column, as the sidebar's agent list, the git panel's tree and the
    /// Command Center grid are.
    FillsColumn,
    /// The git panel's diff: a virtualized list scrolling vertically on its
    /// own state, inside a region that scrolls long lines sideways, with the
    /// vertical bar laid over the outer wrapper.
    DiffLines,
}

struct ScrollProbe {
    shape: Shape,
    rows:  usize,
    list:  ListState,
}

fn row(index: usize, rows: usize) -> Div {
    let row = div().h(px(ROW_HEIGHT)).flex_shrink_0();
    if index + 1 == rows {
        row.debug_selector(|| "last-row".to_string())
    }
    else if index == SIXTH_ROW {
        row.debug_selector(|| "sixth-row".to_string())
    }
    else {
        row
    }
}

/// A row far enough down that drawing it at its place proves the list was
/// given the column's height rather than collapsing.
const SIXTH_ROW: usize = 5;

fn bar(selector: &'static str) -> Div {
    div().h(px(BAR_HEIGHT))
         .flex_shrink_0()
         .debug_selector(move || selector.to_string())
}

impl Render for ScrollProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows;
        let content = v_flex().children((0..rows).map(|index| row(index, rows)));
        match self.shape {
            Shape::Capped => v_flex().size_full()
                                     .child(capped_scroll("capped-list", px(CAP), content))
                                     .child(bar("after-list")),
            Shape::FillsColumn => v_flex().size_full()
                                          .child(bar("header"))
                                          .child(div().id("column-list")
                                                      .flex_1()
                                                      .min_h_0()
                                                      .overflow_y_scrollbar()
                                                      .child(content.min_h_full()))
                                          .child(bar("footer")),
            Shape::DiffLines => {
                v_flex().size_full()
                        .child(bar("header"))
                        .child(div().relative()
                                    .flex_1()
                                    .min_h_0()
                                    .child(div().id("diff-lines")
                                                .size_full()
                                                .overflow_x_scrollbar()
                                                .child(gpui_kit::list(self.list.clone(),
                                                                      move |index, _, _| {
                                                                          row(index, rows)
                                                                              .into_any_element()
                                                                      }).size_full()))
                                    .vertical_scrollbar(&self.list))
            }
        }
    }
}

fn probe(cx: &mut TestAppContext, shape: Shape, rows: usize) -> VisualTestContext {
    let window =
        cx.update(|cx| {
              gpui_kit::init(cx);
              let bounds = Bounds { origin: point(px(0.), px(0.)),
                                    size:   size(px(300.), px(WINDOW_HEIGHT)), };
              cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)),
                                             ..WindowOptions::default() },
                             |window, cx| {
                                 let list =
                                     ListState::new(rows, ListAlignment::Top, px(ROW_HEIGHT));
                                 let view = cx.new(|_| ScrollProbe { shape, rows, list });
                                 cx.new(|cx| Root::new(view, window, cx))
                             })
                .expect("the probe window should open")
          });
    let mut probe_cx = VisualTestContext::from_window(window.into(), cx);
    draw(&mut probe_cx);
    probe_cx
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
          _ = window.draw(cx);
      });
}

/// Scrolls the wheel over a point inside the list.
fn scroll_down(cx: &mut VisualTestContext, at_y: f32) {
    cx.simulate_event(ScrollWheelEvent { position: point(px(20.), px(at_y)),
                                         delta: ScrollDelta::Pixels(point(px(0.), px(-60.))),
                                         ..Default::default() });
    draw(cx);
}

fn top_of(cx: &mut VisualTestContext, selector: &'static str) -> f32 {
    f32::from(cx.debug_bounds(selector)
                .unwrap_or_else(|| panic!("{selector} was not drawn"))
                .top())
}

#[gpui_kit::test]
fn a_capped_list_stays_within_its_cap_and_scrolls(cx: &mut TestAppContext) {
    let mut probe_cx = probe(cx, Shape::Capped, 10);

    assert_eq!(top_of(&mut probe_cx, "after-list"),
               CAP,
               "ten rows overflow the cap, so what follows the list sits at the cap");
    assert!(probe_cx.debug_bounds("scrollbar-overlay").is_some(),
            "the region has a scrollbar layer");

    let before = top_of(&mut probe_cx, "last-row");
    scroll_down(&mut probe_cx, CAP / 2.);
    assert!(top_of(&mut probe_cx, "last-row") < before,
            "the wheel scrolls the capped list");
}

/// A list shorter than its cap takes only its own height, as it did before
/// the wrapper.
#[gpui_kit::test]
fn a_capped_list_shorter_than_its_cap_takes_its_own_height(cx: &mut TestAppContext) {
    let mut probe_cx = probe(cx, Shape::Capped, 2);

    assert_eq!(top_of(&mut probe_cx, "after-list"), 2. * ROW_HEIGHT);
}

#[gpui_kit::test]
fn a_list_filling_its_column_leaves_the_footer_in_place_and_scrolls(cx: &mut TestAppContext) {
    let mut probe_cx = probe(cx, Shape::FillsColumn, 20);

    assert_eq!(top_of(&mut probe_cx, "footer"),
               WINDOW_HEIGHT - BAR_HEIGHT,
               "the list shrinks into the space between header and footer");

    let before = top_of(&mut probe_cx, "last-row");
    scroll_down(&mut probe_cx, WINDOW_HEIGHT / 2.);
    assert!(top_of(&mut probe_cx, "last-row") < before,
            "the wheel scrolls the list");
}

/// Content that fits does not scroll. The Command Center's spec asks for
/// exactly this of a short grid (`openspec/specs/dashboard`).
#[gpui_kit::test]
fn a_list_that_fits_does_not_scroll(cx: &mut TestAppContext) {
    let mut probe_cx = probe(cx, Shape::FillsColumn, 3);

    let before = top_of(&mut probe_cx, "last-row");
    scroll_down(&mut probe_cx, BAR_HEIGHT + ROW_HEIGHT);
    assert_eq!(top_of(&mut probe_cx, "last-row"), before);
}

/// The diff's list keeps the column's height inside the sideways-scrolling
/// region - a list left without a definite height draws no rows at all - and
/// the wheel still scrolls it vertically.
#[gpui_kit::test]
fn a_diff_list_inside_a_sideways_region_fills_it_and_scrolls(cx: &mut TestAppContext) {
    let mut probe_cx = probe(cx, Shape::DiffLines, 40);

    assert_eq!(top_of(&mut probe_cx, "sixth-row"),
               BAR_HEIGHT + SIXTH_ROW as f32 * ROW_HEIGHT,
               "the sixth row is drawn in place, so the list has the column's height");

    let before = top_of(&mut probe_cx, "sixth-row");
    scroll_down(&mut probe_cx, WINDOW_HEIGHT / 2.);
    assert!(top_of(&mut probe_cx, "sixth-row") < before,
            "the wheel scrolls the diff vertically");
}
