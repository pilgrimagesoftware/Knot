//! A click on an issue or change row's actions button opens its menu and
//! does not also run the row's own click - which, on an issue row, opens the
//! browser (`workspace-issues`: "The actions button does not open the
//! issue"). Probed with the real button inside a clickable row, as
//! `tests::pull_request_row_clicks` probes the remove icon.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::component::Root;
use gpui_kit::{
    AppContext, Context, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, WindowOptions,
    div, point, px,
};

use super::row_actions_button;

/// One row: clicking it counts, and the actions button sits at its right end.
struct RowProbe {
    row_clicks: Rc<Cell<usize>>,
}

impl Render for RowProbe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.row_clicks.clone();
        div().id("row")
             .absolute()
             .top(px(0.))
             .left(px(0.))
             .w(px(200.))
             .h(px(40.))
             .child(div().absolute()
                         .top(px(8.))
                         .left(px(160.))
                         .child(row_actions_button("row-actions".into(), |menu, _, _| menu)))
             .on_click(cx.listener(move |_, _, _, _| clicks.set(clicks.get() + 1)))
    }
}

fn probe(cx: &mut TestAppContext) -> (VisualTestContext, Rc<Cell<usize>>) {
    let clicks = Rc::new(Cell::new(0));
    let window = {
        let clicks = clicks.clone();
        cx.update(move |cx| {
              gpui_kit::init(cx);
              cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|_| RowProbe { row_clicks: clicks });
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("the probe window should open")
          })
    };
    let probe_cx = VisualTestContext::from_window(window.into(), cx);
    probe_cx.run_until_parked();
    (probe_cx, clicks)
}

/// The probe itself: a click on the row away from the button reaches it, so
/// the assertion below is not vacuous.
#[gpui_kit::test]
fn a_click_on_the_row_runs_the_row(cx: &mut TestAppContext) {
    let (mut probe_cx, clicks) = probe(cx);
    probe_cx.simulate_click(point(px(20.), px(20.)), Modifiers::default());
    probe_cx.run_until_parked();
    assert_eq!(clicks.get(), 1);
}

#[gpui_kit::test]
fn a_click_on_the_actions_button_does_not_run_the_row(cx: &mut TestAppContext) {
    let (mut probe_cx, clicks) = probe(cx);
    probe_cx.simulate_click(point(px(170.), px(18.)), Modifiers::default());
    probe_cx.run_until_parked();
    assert_eq!(clicks.get(),
               0,
               "the actions button's click reached the row, which would open the issue too");
}
