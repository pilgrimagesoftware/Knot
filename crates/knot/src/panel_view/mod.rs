//! Renders `panel_state::PanelState` as a chat-like panel: streaming
//! messages, tool-call cards (icon by ACP `kind`, body from the call's
//! reported content, with a diff view for diff blocks), and an inline
//! permission prompt. Sibling to `terminal_view.rs` (which renders a
//! `Grid`) per design decision 5 - this renders a completely different
//! data model.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Icon, Sizable, WindowExt};
use gpui_kit::{
    ClickEvent, ClipboardItem, Hsla, InteractiveElement, IntoElement, ListOffset, ListState,
    ParentElement, StatefulInteractiveElement, Styled, div, px, relative, rgb,
};
use knot_acp::{PermissionDecision, PermissionRequest};

use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};
use crate::working_indicator;

mod message;
mod rows;
mod tool_call;

// `rows` is re-exported: `workspace_window` drives the virtualized list
// through it. The other two are imported privately - they are internal to
// this module, and a sibling's `use super::*` picks them up from here.
use message::*;
pub(crate) use rows::*;
use tool_call::*;

/// How much extra space above and below the viewport the list lays out and
/// measures, so scrolling does not pop rows in at the edges.
pub(crate) const LIST_OVERDRAW: f32 = 400.;

const ERROR_COLOR: u32 = 0xEF4444;
const SAFE_COLOR: u32 = 0x22C55E;
const MUTED: u32 = 0x9CA3AF;

/// The panel's render-time styling inputs, grouped rather than passed as
/// two more positional parameters to `render_panel`.
#[derive(Clone, Debug)]
pub(crate) struct PanelStyle {
    pub(crate) permission_risk:    RiskLevel,
    /// The conversation's body text size, from `Settings`'
    /// `markdown_font_size`.
    pub(crate) markdown_font_size: gpui_kit::Pixels,
    /// The theme's monospace family, for tool output and diffs. A real
    /// registered family name is required: `font_family("monospace")` is
    /// not a family GPUI resolves, so it silently fell back to the body
    /// font and shell output rendered proportionally.
    pub(crate) mono_font_family:   gpui_kit::SharedString,
    /// The theme's proportional family, for the panel's own words about a
    /// call - named rather than inherited so a later change setting a
    /// card header monospace cannot sweep the status text along with the
    /// title.
    pub(crate) ui_font_family:     gpui_kit::SharedString,
    /// The theme's danger colour, for a failed tool call's outline.
    pub(crate) danger_color:       Hsla,
    /// The theme's info colour, for a pending or running call's outline.
    pub(crate) info_color:         Hsla,
    /// The theme's ordinary border, the neutral outline a completed call
    /// recedes to.
    pub(crate) border_color:       Hsla,
    /// The raised neutral surface a tool-call card sits on. From the
    /// platform's control background on macOS, so cards track the system
    /// appearance instead of a fixed near-black.
    pub(crate) card_color:         Hsla,
    /// The user prompt bubble's fill - the system accent on macOS - and the
    /// foreground picked to contrast it, so the prompt stays readable in
    /// either appearance and under any accent the user has chosen.
    pub(crate) prompt_color:       Hsla,
    pub(crate) prompt_foreground:  Hsla,
}

impl PanelStyle {
    /// The outline colour a tool call card's status calls for.
    fn outline_color(&self, outline: CardOutline) -> Hsla {
        match outline {
            CardOutline::Danger => self.danger_color,
            CardOutline::Info => self.info_color,
            CardOutline::Neutral => self.border_color,
        }
    }
}

/// The panel's interaction callbacks, grouped rather than threaded through
/// the row renderer and message renderer as four separate parameters. Wrapped
/// in `Rc` so the list's row closure can hand cheap clones to each row it
/// materializes; `PanelState` remains the only thing shared with the ACP
/// reader thread, so this stays on the UI thread.
#[derive(Clone)]
pub(crate) struct PanelCallbacks {
    pub(crate) on_permission_decision: Rc<dyn Fn(PermissionDecision)>,
    pub(crate) on_toggle_track:        Rc<dyn Fn()>,
    pub(crate) on_toggle_tool_call:    Rc<dyn Fn(String)>,
    pub(crate) on_manual_scroll:       Rc<dyn Fn()>,
}

impl PanelCallbacks {
    pub(crate) fn new(on_permission_decision: impl Fn(PermissionDecision) + 'static,
                      on_toggle_track: impl Fn() + 'static,
                      on_toggle_tool_call: impl Fn(String) + 'static,
                      on_manual_scroll: impl Fn() + 'static)
                      -> Self {
        Self { on_permission_decision: Rc::new(on_permission_decision),
               on_toggle_track:        Rc::new(on_toggle_track),
               on_toggle_tool_call:    Rc::new(on_toggle_tool_call),
               on_manual_scroll:       Rc::new(on_manual_scroll), }
    }
}

/// Which of `PanelStyle`'s three outline colours a tool call card takes.
/// Named rather than resolved directly to a colour so the mapping from
/// status is testable without a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardOutline {
    Danger,
    Info,
    Neutral,
}

/// A tool call's outline by status: danger for a failure, info while it
/// is still going, and the panel's neutral border once it is done.
///
/// Completed calls deliberately get no colour of their own - success is
/// the common case, and outlining every finished call leaves nothing
/// standing out. `status` is a plain wire string, so an unrecognized
/// value takes the neutral border rather than being treated as a failure.
fn card_outline(status: &str) -> CardOutline {
    match status {
        "failed" => CardOutline::Danger,
        "pending" | "in_progress" => CardOutline::Info,
        _ => CardOutline::Neutral,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RiskLevel {
    Danger,
    Safe,
    Neutral,
}

pub(crate) fn permission_risk_level(value: &str, name: &str) -> RiskLevel {
    let value = format!("{value} {name}").to_ascii_lowercase();
    if ["bypass", "yolo", "danger"].iter()
                                   .any(|word| value.contains(word))
    {
        RiskLevel::Danger
    }
    else if ["plan", "read"].iter().any(|word| value.contains(word)) {
        RiskLevel::Safe
    }
    else {
        RiskLevel::Neutral
    }
}

pub(crate) fn risk_color(risk: RiskLevel) -> Option<u32> {
    match risk {
        RiskLevel::Danger => Some(ERROR_COLOR),
        RiskLevel::Safe => Some(SAFE_COLOR),
        RiskLevel::Neutral => None,
    }
}

/// Renders the full panel as one virtualized list: every message, then a
/// pending permission prompt or an ended-session banner if applicable.
/// Only rows intersecting the viewport (and a modest overdraw) are laid
/// out and measured, so per-frame cost tracks the pane's size rather than
/// the length of the conversation.
///
/// `list` is the caller's `ListState` and must already hold `row_count`
/// items (see `sync_row_count`); the returned element is the `list`
/// scroller itself, which fills the pane. The callbacks in `callbacks` drive
/// the panel's controls: the permission decision, the auto-scroll toggle for
/// the in-flight response, opening or closing a tool-call card by its id, and
/// a manual jump from a message's action bar - the last so the caller can
/// drop auto-scroll without the reconciler pulling the view back to the tail.
pub(crate) fn render_panel(state: Arc<Mutex<PanelState>>, list: ListState, style: &PanelStyle,
                           callbacks: PanelCallbacks)
                           -> impl IntoElement {
    let row_state = Arc::clone(&state);
    let row_list = list.clone();
    let row_style = style.clone();
    gpui_kit::list(list.clone(), move |index, _window, _cx| {
        let state = row_state.lock().expect("panel state poisoned");
        render_row(index, &state, &row_style, &row_list, &callbacks)
    }).size_full()
      // `min_w_0` so a wide child (a markdown table, a long command line)
      // clips instead of stretching the pane and pushing the input row's
      // Send button off screen, per `knot-ui-conventions.md`'s "Flex
      // overflow" rule.
      .min_w_0()
      // Only vertical padding survives on the list itself: the virtualizer
      // lays each row out at the full viewport width and paints it at x=0,
      // so `px_*` here would be ignored and the sides would collapse. The
      // side inset and the gap between rows live on each row instead (see
      // `render_row`); top and bottom padding is honoured here.
      .pt_2()
      .pb_2()
      // Set the body size once, here, and let it cascade: a size applied to
      // the `TextView` itself reaches its paint but not the line wrapper's
      // measuring pass, so runs got measured at one size and drawn at
      // another and overlapped each other - worst around inline code, which
      // is measured separately in the mono family.
      .text_size(style.markdown_font_size)
}

/// Renders a single list row by resolving it against the current panel
/// state. An index past the end draws nothing rather than panicking, so a
/// frame racing a `sync_row_count` splice cannot crash the window.
///
/// Each row carries its own side inset and vertical margin: the virtualizer
/// ignores the list's horizontal padding and has no gap concept, so this is
/// where the content gets its breathing room from the pane edges and from
/// neighbouring rows.
fn render_row(index: usize, state: &PanelState, style: &PanelStyle, list: &ListState,
              callbacks: &PanelCallbacks)
              -> gpui_kit::AnyElement {
    let row = match row_at(state, index) {
        Some(PanelRow::Message(message_index)) => {
            let last_index = state.messages.len().checked_sub(1);
            let message = &state.messages[message_index];
            render_message(Message { state,
                                     index: message_index,
                                     is_last: Some(message_index) == last_index,
                                     style,
                                     list },
                           message,
                           callbacks)
        }
        Some(PanelRow::Permission) => {
            let request = state.pending_permission
                               .as_ref()
                               .expect("row_at yields Permission only while a request is pending");
            render_permission_prompt(state,
                                     request,
                                     style.permission_risk,
                                     callbacks.on_permission_decision.clone()).into_any_element()
        }
        Some(PanelRow::Ended) => {
            let cause = state.ended
                             .as_ref()
                             .expect("row_at yields Ended only once the session has ended");
            render_ended_banner(cause).into_any_element()
        }
        Some(PanelRow::Working) => {
            working_indicator::render(knot_agents::AgentState::Running, true).into_any_element()
        }
        None => return div().into_any_element(),
    };
    div().w_full()
         .min_w_0()
         .px_4()
         .py_2()
         .child(row)
         .into_any_element()
}

/// An inline permission request with actionable allow/deny controls, per
/// `acp-panel-ui`'s permission-prompts requirement. Sending further
/// prompts is blocked by the caller while this is rendered (the caller
/// checks `PanelState::pending_permission` before calling `prompt`).
fn render_permission_prompt(panel_state: &PanelState, request: &PermissionRequest,
                            permission_risk: RiskLevel,
                            on_decision: Rc<dyn Fn(PermissionDecision)>)
                            -> impl IntoElement {
    let allow = on_decision.clone();
    let deny = on_decision;
    v_flex()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(risk_color(permission_risk).unwrap_or(0x3B82F6)))
        .child(div().text_sm().child(format!(
            "Permission requested for {}",
            panel_state.display_name(request)
        )))
        .child(
            h_flex()
                .gap_2()
                .child(
                    Button::new("panel-permission-allow")
                        .label("Allow")
                        .primary()
                        .small()
                        .on_click(move |_: &ClickEvent, _, _| {
                            allow(PermissionDecision::Allow);
                        }),
                )
                .child(
                    Button::new("panel-permission-deny")
                        .label("Deny")
                        .ghost()
                        .small()
                        .on_click(move |_: &ClickEvent, _, _| {
                            deny(PermissionDecision::Deny);
                        }),
                ),
        )
}

fn render_ended_banner(cause: &knot_acp::SessionEndCause) -> impl IntoElement {
    div().text_xs()
         .text_color(rgb(ERROR_COLOR))
         .child(format!("Session ended: {cause}"))
}

#[cfg(test)]
mod tests;
