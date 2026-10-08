//! A running tool call keeps animating (#499): its card title and, in
//! compact mode, its run's summary line request frame after frame from
//! inside the conversation's virtualized `list()`, and stop once the call
//! finishes.
//!
//! GPUI's test platform does not rasterize, so these assert the animation's
//! engine - a next frame scheduled, frame after frame - rather than pixels.
//! The finished case is the control: with nothing running, nothing in the
//! same row asks for frames, so the frames counted come from the shimmer.

use std::rc::Rc;

use gpui_kit::component::Root;
use gpui_kit::{
    AppContext, Context, Hsla, IntoElement, ListAlignment, ListState, ParentElement, Render,
    Styled, TestAppContext, VisualTestContext, Window, WindowOptions, div, px,
};

use super::style::{PanelStyle, RiskLevel};
use super::summary_row::render_tool_run_summary;
use super::tool_call::render_tool_call_card;
use crate::panel_state::{ToolCallCard, ToolRunSummary};

fn style() -> PanelStyle {
    PanelStyle { permission_risk:    RiskLevel::Neutral,
                 markdown_font_size: px(14.),
                 mono_font_family:   "Menlo".into(),
                 ui_font_family:     "Helvetica".into(),
                 title_font_family:  "Helvetica".into(),
                 danger_color:       Hsla::default(),
                 info_color:         Hsla::default(),
                 border_color:       Hsla::default(),
                 card_color:         Hsla::default(),
                 prompt_color:       Hsla::default(),
                 prompt_foreground:  Hsla::default(),
                 compact_tool_calls: true, }
}

fn card(status: &str) -> ToolCallCard {
    ToolCallCard { id:        "tc1".to_owned(),
                   kind:      "execute".to_owned(),
                   title:     "sleep 30".to_owned(),
                   status:    status.to_owned(),
                   content:   Vec::new(),
                   raw_input: None,
                   meta:      None, }
}

/// What the probe's one list row draws.
#[derive(Clone, Copy)]
enum Row {
    Card { finished: bool },
    Summary { running: bool },
}

struct Probe {
    list: ListState,
    row:  Row,
}

impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let row = self.row;
        div().size_full()
             .child(gpui_kit::list(self.list.clone(), move |_, _, _| match row {
                        Row::Card { finished } => {
                            let status = if finished { "completed" } else { "in_progress" };
                            render_tool_call_card(&card(status), &style(), true, Rc::new(|_| {}))
                                .into_any_element()
                        }
                        Row::Summary { running } => {
                            let summary = ToolRunSummary { calls: 2,
                                                           commands_run: 2,
                                                           running,
                                                           ..ToolRunSummary::default() };
                            render_tool_run_summary(summary,
                                                    "tc1".to_owned(),
                                                    &style(),
                                                    Rc::new(|_| {})).into_any_element()
                        }
                    }).size_full())
    }
}

/// How many next-frame callbacks each of three consecutive frames scheduled.
fn frames_requested(row: Row, cx: &mut TestAppContext) -> [usize; 3] {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let list = ListState::new(1, ListAlignment::Top, px(100.));
                             let view = cx.new(|_| Probe { list, row });
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("the probe window should open")
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    std::array::from_fn(|_| {
        cx.update(|window, cx| {
              let _ = window.draw(cx);
              window.simulate_next_frame(cx)
          })
    })
}

#[gpui_kit::test]
fn a_running_card_keeps_requesting_frames(cx: &mut TestAppContext) {
    let frames = frames_requested(Row::Card { finished: false }, cx);
    assert!(frames.iter().all(|&count| count > 0),
            "a running card's title stopped animating: {frames:?}");
}

#[gpui_kit::test]
fn a_finished_card_requests_no_frames(cx: &mut TestAppContext) {
    assert_eq!(frames_requested(Row::Card { finished: true }, cx),
               [0, 0, 0]);
}

/// Compact mode, where the user saw nothing move: the summary line of a run
/// with a call still going animates.
#[gpui_kit::test]
fn a_running_summary_keeps_requesting_frames(cx: &mut TestAppContext) {
    let frames = frames_requested(Row::Summary { running: true }, cx);
    assert!(frames.iter().all(|&count| count > 0),
            "a running run's summary line does not animate: {frames:?}");
}

#[gpui_kit::test]
fn a_finished_summary_requests_no_frames(cx: &mut TestAppContext) {
    assert_eq!(frames_requested(Row::Summary { running: false }, cx),
               [0, 0, 0]);
}
