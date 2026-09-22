//! The plan pane: a committed task graph drawn as a diagram.
//!
//! Contract: `openspec/specs/task-graph/spec.md`'s "A committed plan is
//! shown to the user".
//!
//! Reads `Agent::mermaid_source`, which until now nothing rendered - the
//! `view-mermaid` tool wrote it and it stopped there. So this also makes
//! that tool do something visible, for every agent and not just an
//! orchestrator's plan.
//!
//! Split three ways on purpose. [`mermaid`] recovers the graph from the
//! text and [`layout`] arranges it, both pure and both tested without a
//! window; this module only turns the arrangement into elements. Layout
//! bugs are the likely ones, and they are the ones a test can catch.

pub mod layout;
pub mod mermaid;

use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, FontWeight, Hsla, IntoElement, ParentElement, PathBuilder, Styled, canvas,
    div, point, px,
};

use crate::plan_view::layout::{CARD_HEIGHT, CARD_WIDTH, Edge, Layout};
use crate::plan_view::mermaid::PlanGraph;

/// How far an edge's control point reaches horizontally, as a fraction of
/// the gap it spans. Enough curve to separate edges that share an anchor,
/// not so much that a short hop loops.
const EDGE_CURVE: f32 = 0.5;
const EDGE_WIDTH: f32 = 1.5;
const ARROWHEAD: f32 = 5.;

/// The accent for a task state. Unknown states - a class this build does
/// not know - draw neutral rather than being dropped, so a diagram from a
/// newer emitter still reads.
fn state_color(state: Option<&str>, cx: &App) -> Hsla {
    match state {
        Some("done") => cx.theme().success,
        Some("failed") | Some("blocked") => cx.theme().danger,
        Some("dispatched") => cx.theme().accent,
        Some("ready") => cx.theme().info,
        _ => cx.theme().muted_foreground,
    }
}

/// One task, as a card at the position the layout gave it.
fn card(node: &mermaid::PlanNode, left: f32, top: f32, cx: &App) -> AnyElement {
    let accent = state_color(node.state.as_deref(), cx);
    div().absolute()
         .left(px(left))
         .top(px(top))
         .w(px(CARD_WIDTH))
         .h(px(CARD_HEIGHT))
         .flex()
         .rounded_md()
         .overflow_hidden()
         .border_1()
         .border_color(cx.theme().border)
         .bg(cx.theme().secondary)
         // The state reads as a stripe down the leading edge rather than
         // as a fill: a filled card competes with its own label, and the
         // stripes line up into a column that can be scanned.
         .child(div().w(px(4.)).h_full().flex_shrink_0().bg(accent))
         .child(div().flex_1()
                     .min_w_0()
                     .px_2()
                     .py_1()
                     .child(div().text_xs()
                                 .font_weight(FontWeight::MEDIUM)
                                 .text_color(cx.theme().foreground)
                                 .child(node.label.clone()))
                     .child(div().text_xs()
                                 .text_color(accent)
                                 .child(node.state.clone().unwrap_or_default())))
         .into_any_element()
}

/// Paints every dependency as a curve from one card's right edge to the
/// next card's left edge, with a filled arrowhead at the target.
fn edges(edge_list: Vec<Edge>, color: Hsla) -> AnyElement {
    canvas(move |_, _, _| {
               edge_list.iter()
                        .filter_map(|edge| {
                            let (from_x, from_y) = edge.from;
                            let (to_x, to_y) = edge.to;
                            let reach = (to_x - from_x).abs() * EDGE_CURVE;

                            let mut line = PathBuilder::stroke(px(EDGE_WIDTH));
                            line.move_to(point(px(from_x), px(from_y)));
                            line.cubic_bezier_to(point(px(to_x - ARROWHEAD), px(to_y)),
                                                 point(px(from_x + reach), px(from_y)),
                                                 point(px(to_x - reach), px(to_y)));

                            let mut head = PathBuilder::fill();
                            head.add_polygon(&[point(px(to_x), px(to_y)),
                                               point(px(to_x - ARROWHEAD * 1.6),
                                                     px(to_y - ARROWHEAD * 0.8)),
                                               point(px(to_x - ARROWHEAD * 1.6),
                                                     px(to_y + ARROWHEAD * 0.8))],
                                             true);
                            Some((line.build().ok()?, head.build().ok()?))
                        })
                        .collect::<Vec<_>>()
           },
           move |_, paths, window, _| {
               for (line, head) in paths {
                   window.paint_path(line, color);
                   window.paint_path(head, color);
               }
           }).absolute()
             .size_full()
             .into_any_element()
}

/// The diagram for `source`, or `None` when there is nothing to draw.
///
/// `None` rather than an empty box: the caller decides what an empty plan
/// looks like, and a blank pane where a diagram should be reads as a bug.
pub fn plan_diagram(source: &str, cx: &App) -> Option<AnyElement> {
    let graph: PlanGraph = mermaid::parse(source);
    if graph.is_empty() {
        return None;
    }
    let laid: Layout = layout::layout(&graph);
    let mut canvas_layer =
        div().relative()
             .w(px(laid.width))
             .h(px(laid.height))
             // Edges first, so a card always draws over the line that
             // reaches it rather than being crossed by it.
             .child(edges(laid.edges.clone(), cx.theme().muted_foreground.opacity(0.7)));
    for placed in &laid.cards {
        canvas_layer =
            canvas_layer.child(card(&graph.nodes[placed.node], placed.left, placed.top, cx));
    }
    Some(canvas_layer.into_any_element())
}
