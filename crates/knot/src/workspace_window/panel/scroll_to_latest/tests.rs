//! The control over a row that would take any click it is handed.

use std::cell::Cell;
use std::rc::Rc;

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
use gpui_kit::div;

use super::scroll_to_latest;

/// A full-size clickable row - standing in for a collapsed tool call - with
/// the control floating over it, counting the clicks each one receives.
struct Overlaid {
    row_clicks:    Rc<Cell<usize>>,
    control_jumps: Rc<Cell<usize>>,
}

impl Render for Overlaid {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let row_clicks = Rc::clone(&self.row_clicks);
        let control_jumps = Rc::clone(&self.control_jumps);
        div().relative()
             .size_full()
             .child(div().id("row")
                         .size_full()
                         .on_click(move |_, _, _| row_clicks.set(row_clicks.get() + 1)))
             .child(scroll_to_latest(move |_| control_jumps.set(control_jumps.get() + 1)))
    }
}

/// #594: the click is the control's, and the row under it never sees it.
#[gpui_kit::test]
fn clicking_the_control_does_not_reach_the_row_beneath_it(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let row_clicks = Rc::new(Cell::new(0));
    let control_jumps = Rc::new(Cell::new(0));
    let window = cx.add_window({
                       let row_clicks = Rc::clone(&row_clicks);
                       let control_jumps = Rc::clone(&control_jumps);
                       |_, _| Overlaid { row_clicks,
                                         control_jumps }
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let control = cx.debug_bounds("panel-scroll-to-latest")
                    .expect("the control is drawn");
    cx.simulate_click(control.center(), Modifiers::none());

    assert_eq!(control_jumps.get(), 1, "the control takes the click");
    assert_eq!(row_clicks.get(), 0, "the row beneath it must not");
}
