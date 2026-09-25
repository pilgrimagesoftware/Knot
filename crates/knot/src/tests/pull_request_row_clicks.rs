//! A control nested inside a clickable row runs only its own action
//! (`openspec/specs/pull-request-tracking`, "The user can remove a recorded
//! pull request").
//!
//! The Pull Requests view is the one place in `render/` that puts a clickable
//! control inside a clickable row: the whole row opens the pull request, and
//! the trash icon within it removes the record. GPUI fires click handlers in
//! the bubble phase, so without `stop_propagation` the row's handler runs
//! after the control's and the user gets both actions from one click.
//!
//! These pin the dependency property the fix rests on rather than the row's
//! own markup, in the same spirit as `pane_focus`'s dialog probe: what makes
//! the fix necessary is how gpui dispatches, and that is what would change
//! under it without anything in this crate being edited.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::Modifiers;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;

/// Counts of which handlers a click reached.
#[derive(Clone, Default)]
struct Hits {
    row:     Rc<Cell<usize>>,
    control: Rc<Cell<usize>>,
}

/// Stands in for one Pull Requests row: an outer element that opens the pull
/// request, and a control inside it that removes the record. The shape is
/// `render/pull_requests_pane.rs`'s `render_row`, reduced to the two click
/// handlers and the nesting between them.
struct RowProbe {
    hits:  Hits,
    /// Whether the inner control stops the click from bubbling - the fix
    /// under test. Both settings are exercised, so the test says what the
    /// dispatch does either way rather than only that the fix works.
    stops: bool,
}

impl Render for RowProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row = self.hits.row.clone();
        let control = self.hits.control.clone();
        let stops = self.stops;
        div().id("row")
             .absolute()
             .top(px(0.))
             .left(px(0.))
             .w(px(200.))
             .h(px(40.))
             .child(div().id("control")
                         .absolute()
                         .top(px(0.))
                         .left(px(160.))
                         .w(px(40.))
                         .h(px(40.))
                         .on_click(move |_, _window, cx| {
                             control.set(control.get() + 1);
                             if stops {
                                 cx.stop_propagation();
                             }
                         }))
             .on_click(cx.listener(move |_view, _, _window, _cx| {
                             row.set(row.get() + 1);
                         }))
    }
}

fn probe_window(stops: bool, cx: &mut TestAppContext) -> (VisualTestContext, Hits) {
    let hits = Hits::default();
    let window = {
        let hits = hits.clone();
        cx.update(move |cx| {
              gpui_kit::init(cx);
              cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|_| RowProbe { hits, stops });
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("the probe window should open")
          })
    };
    (VisualTestContext::from_window(window.into(), cx), hits)
}

/// The bug, stated as the behaviour that produces it: a click on the inner
/// control reaches the row's handler too, so removing a pull request also
/// opens it.
///
/// If this ever fails, gpui has stopped bubbling clicks to ancestors and the
/// `stop_propagation` in `render_row` is dead weight rather than load-bearing.
#[gpui_kit::test]
fn a_click_on_an_inner_control_reaches_the_row_without_the_guard(cx: &mut TestAppContext) {
    let (mut probe_cx, hits) = probe_window(false, cx);
    probe_cx.run_until_parked();

    probe_cx.simulate_click(point(px(180.), px(20.)), Modifiers::default());
    probe_cx.run_until_parked();

    assert_eq!(hits.control.get(),
               1,
               "the control's own handler should run");
    assert_eq!(hits.row.get(),
               1,
               "without the guard the click bubbles to the row - this is the defect the guard \
                exists for");
}

/// The fix: the control's handler stops the click, so the row never sees it.
#[gpui_kit::test]
fn the_guard_keeps_the_click_from_reaching_the_row(cx: &mut TestAppContext) {
    let (mut probe_cx, hits) = probe_window(true, cx);
    probe_cx.run_until_parked();

    probe_cx.simulate_click(point(px(180.), px(20.)), Modifiers::default());
    probe_cx.run_until_parked();

    assert_eq!(hits.control.get(),
               1,
               "the control's own handler should still run");
    assert_eq!(hits.row.get(),
               0,
               "stopping propagation should keep the row's handler from running, so removing a \
                pull request does not also open it");
}

/// The guard must not cost the row its own clicks: a click that lands on the
/// row away from the control still opens the pull request.
#[gpui_kit::test]
fn the_row_still_takes_clicks_outside_the_control(cx: &mut TestAppContext) {
    let (mut probe_cx, hits) = probe_window(true, cx);
    probe_cx.run_until_parked();

    probe_cx.simulate_click(point(px(40.), px(20.)), Modifiers::default());
    probe_cx.run_until_parked();

    assert_eq!(hits.control.get(), 0, "the control was not clicked");
    assert_eq!(hits.row.get(), 1, "the row's own handler should still run");
}

/// A row's context menu opens on a secondary click, and that click must not
/// also open the pull request. `on_click` is primary-button only, which is
/// what `render_row` relies on by adding the menu without a guard.
#[gpui_kit::test]
fn a_secondary_click_does_not_reach_the_rows_click_handler(cx: &mut TestAppContext) {
    let (mut probe_cx, hits) = probe_window(false, cx);
    probe_cx.run_until_parked();

    let at = point(px(20.), px(20.));
    probe_cx.simulate_mouse_down(at, gpui_kit::MouseButton::Right, Modifiers::default());
    probe_cx.simulate_mouse_up(at, gpui_kit::MouseButton::Right, Modifiers::default());
    probe_cx.run_until_parked();

    assert_eq!(hits.row.get(),
               0,
               "a secondary click must not open the pull request");
}
