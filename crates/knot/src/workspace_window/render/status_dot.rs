//! The sidebar's state dot, shared by the full and compact rows so its
//! colour, size and tooltip can't drift apart between the two. Where the dot
//! sits is the caller's: beside the text on the full row, over the avatar's
//! corner on the compact one.

use std::time::SystemTime;

use gpui_kit::InteractiveElement;
use gpui_kit::Stateful;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use crate::app_state::state_color;
use crate::app_state::state_tooltip;

/// Agent `id`'s state dot. Hovering it names the state, as the header does
/// beside its own dot, and for an idle agent says when its last message was
/// (#582). The text is built when the tooltip opens rather than at render,
/// so a "5m ago" read an hour later still says an hour.
pub(super) fn status_dot(id: Uuid, state: knot_agents::AgentState,
                         idle_since: Option<SystemTime>)
                         -> Stateful<gpui_kit::Div> {
    div().id(gpui_kit::ElementId::from(format!("agent-status-dot-{id}")))
         .debug_selector(|| "agent-status-dot".into())
         .flex_shrink_0()
         .w(px(8.))
         .h(px(8.))
         .rounded_full()
         .bg(state_color(state))
         .tooltip(move |window, cx| {
             Tooltip::new(state_tooltip(state, idle_since)).build(window, cx)
         })
}
