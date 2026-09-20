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
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{
    ClickEvent, ClipboardItem, Hsla, InteractiveElement, IntoElement, ListOffset, ListState,
    ParentElement, StatefulInteractiveElement, Styled, div, px, relative, rgb,
};
use knot_acp::{PermissionDecision, PermissionRequest};

use crate::panel_state::{PanelMessage, PanelState, ToolCallCard};

/// How much extra space above and below the viewport the list lays out and
/// measures, so scrolling does not pop rows in at the edges.
pub(crate) const LIST_OVERDRAW: f32 = 400.;

const CARD_BG: u32 = 0x1E1E1E;
const CARD_BORDER: u32 = 0x333333;
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

/// One virtualized row of the conversation: every message in order, then
/// the pending permission prompt, then the ended-session banner. Modelling
/// the trailing cards as rows of the same list (rather than siblings below
/// a scroller) keeps them inside the virtualizer and lets tail-following
/// track them like any other row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PanelRow {
    /// `PanelState::messages[index]`.
    Message(usize),
    /// The pending permission prompt, when one is awaiting a decision.
    Permission,
    /// The ended-session banner, when the session has ended.
    Ended,
}

/// The number of rows `state` needs the conversation list to lay out -
/// every message, plus a permission row and/or an ended row when present.
pub(crate) fn row_count(state: &PanelState) -> usize {
    state.messages.len()
    + usize::from(state.pending_permission.is_some())
    + usize::from(state.ended.is_some())
}

/// Resolves a list index to the row it draws. `None` for an index past the
/// end, which the list can briefly hold between a row landing in
/// `PanelState` and the next `sync_row_count` splice.
pub(crate) fn row_at(state: &PanelState, index: usize) -> Option<PanelRow> {
    let messages = state.messages.len();
    let has_permission = state.pending_permission.is_some();
    if index < messages {
        Some(PanelRow::Message(index))
    }
    else if index == messages && has_permission {
        Some(PanelRow::Permission)
    }
    else if index == messages + usize::from(has_permission) && state.ended.is_some() {
        Some(PanelRow::Ended)
    }
    else {
        None
    }
}

/// Brings the list's item count in step with `state`'s rows, splicing only
/// the delta so off-screen rows keep the heights they were already
/// measured at and the reader's scroll position is left alone. `known` is
/// the count the list currently holds, and the returned count should be
/// passed back as `known` next frame.
pub(crate) fn sync_row_count(list: &ListState, known: usize, state: &PanelState) -> usize {
    let count = row_count(state);
    if count > known {
        list.splice(known..known, count - known);
    }
    else if count < known {
        list.splice(count..known, 0);
    }
    count
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
            render_permission_prompt(request,
                                     style.permission_risk,
                                     callbacks.on_permission_decision.clone()).into_any_element()
        }
        Some(PanelRow::Ended) => {
            let cause = state.ended
                             .as_ref()
                             .expect("row_at yields Ended only once the session has ended");
            render_ended_banner(cause).into_any_element()
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

/// One message's render inputs, grouped so `render_message` keeps a short
/// parameter list as the panel gains styling.
struct Message<'a> {
    state:   &'a PanelState,
    index:   usize,
    is_last: bool,
    style:   &'a PanelStyle,
    list:    &'a ListState,
}

fn render_message(ctx: Message<'_>, message: &PanelMessage, callbacks: &PanelCallbacks)
                  -> gpui_kit::AnyElement {
    let Message { state,
                  index,
                  is_last,
                  style,
                  list, } = ctx;
    match message {
        // Right-aligned, tinted background - visually distinct from the
        // assistant's plain left-aligned text, per acp-panel-ui's
        // "visually distinguish user messages, assistant messages, and
        // system/tool content" requirement.
        PanelMessage::User { text, queued } => h_flex().w_full()
                                            .min_w_0()
                                            .justify_end()
                                            .child(div().max_w(relative(0.85))
                                                        .min_w_0()
                                                        .text_sm()
                                                        .text_color(rgb(if *queued { 0x9CA3AF } else { 0xFFFFFF }))
                                                        .px_3()
                                                        .py_1p5()
                                                        .rounded_md()
                                                        .bg(rgb(if *queued { 0x374151 } else { 0x2563EB }))
                                                        .child(v_flex().gap_1()
                                                                    .children((*queued).then(|| div().text_xs().text_color(rgb(0xD1D5DB)).child(knot_core::l10n::t("panel.queued"))))
                                                                    .child(text.clone())))
                                            .into_any_element(),
        PanelMessage::Assistant(text) => {
            v_flex().w_full()
                    .min_w_0()
                    .gap_1()
                    // Plain `w_full().min_w_0()`, deliberately *not* a
                    // scroll container: a scroll parent hands its child an
                    // unconstrained width, so the markdown measured its
                    // runs against one width and painted them into
                    // another, drawing words on top of each other. Wide
                    // content clips here instead, which `min_w_0` at least
                    // keeps from stretching the pane.
                    .child(div().w_full()
                                .min_w_0()
                                .child(TextView::markdown(("panel-message-markdown",
                                                           index as u64),
                                                          text.clone())))
                    .children((is_last && state.turn_active).then(|| {
                                                                render_track_toggle(state.tracking,
                                                      callbacks.on_toggle_track.clone())
                                                            }))
                    .children((!(is_last && state.turn_active)).then(|| {
                                  let user_index = preceding_user_message(state, index);
                                  render_response_actions(text.clone(),
                                                          user_index,
                                                          index,
                                                          list,
                                                          callbacks.on_manual_scroll.clone())
                              }))
                    .into_any_element()
        }
        PanelMessage::ToolCall(card) => render_tool_call_card(card,
                                                              style,
                                                              state.is_collapsed(card),
                                                              callbacks.on_toggle_tool_call
                                                                       .clone()).into_any_element(),
        // Left-aligned like the assistant's own text, since it stands
        // where that answer would have been, but in the error color and
        // outlined so it doesn't read as something the agent said.
        PanelMessage::Error(text) => div().w_full()
                                          .min_w_0()
                                          .text_sm()
                                          .text_color(rgb(ERROR_COLOR))
                                          .px_3()
                                          .py_1p5()
                                          .rounded_md()
                                          .border_1()
                                          .border_color(rgb(ERROR_COLOR))
                                          .child(text.clone())
                                          .into_any_element(),
    }
}

/// The index of the nearest `PanelMessage::User` before `index`, for the
/// response action bar's "scroll to user input" control.
fn preceding_user_message(state: &PanelState, index: usize) -> Option<usize> {
    state.messages[..index].iter()
                           .rposition(|message| matches!(message, PanelMessage::User { .. }))
}

/// The in-flight response's auto-scroll toggle, per the track toggle
/// design's per-response scope - shown only on the currently streaming
/// response, replaced by the response action bar once it finalizes.
fn render_track_toggle(tracking: bool, on_toggle: Rc<dyn Fn()>) -> impl IntoElement {
    h_flex().child(Button::new("panel-track-toggle").icon(if tracking {
                                                              IconName::CircleDot
                                                          }
                                                          else {
                                                              IconName::Circle
                                                          })
                                                    .tooltip(if tracking {
                                                                 "Following new output"
                                                             }
                                                             else {
                                                                 "Not following new output"
                                                             })
                                                    .ghost()
                                                    .small()
                                                    .on_click(move |_: &ClickEvent, _, _| {
                                                        on_toggle()
                                                    }))
}

/// A finalized response's action bar: copy, scroll to the user message that
/// prompted it, and scroll to the top of the conversation.
///
/// The two scroll buttons drive the `ListState` directly (rather than a
/// `ScrollHandle`) so they also stop tail-following - `scroll_to` on an
/// earlier item does that on its own - and then call `on_manual_scroll` so
/// the caller clears its own tracking flag, keeping the reconciler from
/// pulling the view straight back to the tail. Button ids carry `index` so
/// each response's bar is a distinct hit target under virtualization.
fn render_response_actions(text: String, user_index: Option<usize>, index: usize,
                           list: &ListState, on_manual_scroll: Rc<dyn Fn()>)
                           -> impl IntoElement {
    let scroll_to_user = list.clone();
    let scroll_to_top = list.clone();
    let manual_to_user = on_manual_scroll.clone();
    let manual_to_top = on_manual_scroll;
    h_flex()
        .gap_1()
        .child(
            Button::new(("panel-copy-response", index as u64))
                .icon(IconName::Copy)
                .tooltip("Copy response")
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                }),
        )
        .children(user_index.map(|user_index| {
            Button::new(("panel-scroll-to-user", index as u64))
                .icon(IconName::ArrowUp)
                .tooltip("Scroll to your message")
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, _, _| {
                    scroll_to_user.scroll_to(ListOffset {
                        item_ix: user_index,
                        offset_in_item: px(0.),
                    });
                    manual_to_user();
                })
        }))
        .child(
            Button::new(("panel-scroll-to-top", index as u64))
                .icon(IconName::ChevronsUp)
                .tooltip("Scroll to top")
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, _, _| {
                    scroll_to_top.scroll_to(ListOffset {
                        item_ix: 0,
                        offset_in_item: px(0.),
                    });
                    manual_to_top();
                }),
        )
}

/// A tool-call card: an icon/title/status header over whatever content
/// the agent has reported so far - diff blocks as an added/removed line
/// view, everything else as monospace output (`render_tool_call_body`).
/// The header separates content from chrome by font: the title is a
/// command, a path or an identifier and renders monospace, the status is
/// the panel's own word for the call and stays proportional. The outline
/// is coloured by `card_outline`.
///
/// `collapsed` comes from `PanelState::is_collapsed`, so a succeeded call
/// folds to just this header. The whole header row is the toggle's hit
/// target, not only the chevron: a 12px icon is a poor one and the header
/// carries no other action (design decision "The control is a disclosure
/// chevron, and the whole header toggles").
fn render_tool_call_card(card: &ToolCallCard, style: &PanelStyle, collapsed: bool,
                         on_toggle: Rc<dyn Fn(String)>)
                         -> impl IntoElement {
    let label = if card.title.is_empty() {
        card.kind.clone()
    }
    else {
        card.title.clone()
    };
    let id = card.id.clone();
    v_flex().w_full()
            .min_w_0()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(style.outline_color(card_outline(&card.status)))
            .bg(rgb(CARD_BG))
            .child(h_flex().id(("panel-tool-call-header", element_id(&card.id)))
                           .w_full()
                           .min_w_0()
                           .gap_2()
                           .items_center()
                           .cursor_pointer()
                           .on_click(move |_: &ClickEvent, _, _| on_toggle(id.clone()))
                           .child(Icon::new(disclosure_icon(collapsed)).xsmall()
                                                                       .text_color(rgb(MUTED)))
                           .child(Icon::new(tool_call_icon(&card.kind)).xsmall()
                                                                       .text_color(rgb(MUTED)))
                           .child(div().flex_1()
                                       .min_w_0()
                                       .font_family(style.mono_font_family.clone())
                                       .text_xs()
                                       .text_color(rgb(MUTED))
                                       .child(label))
                           .child(div().flex_shrink_0()
                                       .font_family(style.ui_font_family.clone())
                                       .text_xs()
                                       .text_color(rgb(if card.failed() {
                                                       ERROR_COLOR
                                                   }
                                                   else {
                                                       MUTED
                                                   }))
                                       .child(status_label(&card.status))))
            .children((!collapsed).then(|| render_tool_call_body(card, style)))
}

/// A card's content, drawn only while the card is expanded. An unfinished
/// call with no content yet shows an in-progress placeholder; a *finished*
/// one with no content shows nothing rather than a stale "Running…".
fn render_tool_call_body(card: &ToolCallCard, style: &PanelStyle) -> impl IntoElement {
    v_flex().w_full()
            .min_w_0()
            .gap_2()
            .children(card.content
                          .iter()
                          .map(|content| render_tool_call_content(content, style)))
            .children((card.content.is_empty() && !card.is_finished())
                                                                     .then(in_progress_placeholder))
}

/// The disclosure chevron for a card in either state: pointing right at a
/// collapsed card (its content is off to the side, unopened) and down at
/// an expanded one (its content is below), the platform convention.
fn disclosure_icon(collapsed: bool) -> IconName {
    if collapsed {
        IconName::ChevronRight
    }
    else {
        IconName::ChevronDown
    }
}

/// A stable element id for a tool call's header. GPUI element ids are
/// `&'static str` or an integer, and a tool-call id is neither, so it is
/// hashed - collisions only cost the wrong card's click state, and within
/// one conversation they are not realistic.
fn element_id(tool_call_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    tool_call_id.hash(&mut hasher);
    hasher.finish()
}

/// One tool-call content block. Diffs get the added/removed line view
/// `acp-panel-ui`'s tool-call-rendering requirement asks for; text and
/// terminal references fall back to monospace output.
fn render_tool_call_content(content: &knot_acp::ToolCallContent, style: &PanelStyle)
                            -> gpui_kit::AnyElement {
    match content {
        knot_acp::ToolCallContent::Diff { path,
                                          old_text,
                                          new_text, } => {
            render_diff(path, old_text.as_deref(), new_text, style).into_any_element()
        }
        knot_acp::ToolCallContent::Text(text) => render_output_text(text, style).into_any_element(),
        knot_acp::ToolCallContent::Terminal { terminal_id } => {
            render_output_text(&format!("[terminal {terminal_id}]"), style).into_any_element()
        }
    }
}

/// Human-readable form of an ACP tool-call status. Unrecognized statuses
/// pass through unchanged rather than being swallowed - `status` is a
/// plain wire string, and a future value is more useful shown than hidden.
fn status_label(status: &str) -> String {
    match status {
        "pending" => "Pending".to_string(),
        "in_progress" => "Running…".to_string(),
        "completed" => "Done".to_string(),
        "failed" => "Failed".to_string(),
        other => other.to_string(),
    }
}

/// Maps an ACP tool-call `kind` to an identifying icon, per the response
/// action bar design's "icon lookup keyed on kind" decision. `kind` is a
/// plain string off the wire rather than a closed enum, so the match is
/// string-keyed with a generic fallback arm rather than truly exhaustive.
fn tool_call_icon(kind: &str) -> IconName {
    match kind {
        "read" => IconName::FileText,
        "edit" => IconName::Pencil,
        "delete" => IconName::Trash,
        "move" => IconName::Move,
        "search" => IconName::Search,
        "execute" => IconName::Terminal,
        "think" => IconName::Brain,
        "fetch" => IconName::Globe,
        _ => IconName::Wrench,
    }
}

fn in_progress_placeholder() -> impl IntoElement {
    div().text_xs().text_color(rgb(MUTED)).child("Running…")
}

/// A file-edit diff as an added/removed line view rather than raw text,
/// per `acp-panel-ui`'s tool-call-rendering requirement. `old_text` is
/// `None` for a newly created file, in which case every line is an
/// addition.
fn render_diff(path: &str, old_text: Option<&str>, new_text: &str, style: &PanelStyle)
               -> impl IntoElement {
    v_flex().w_full()
            .min_w_0()
            .gap_1()
            .child(div().w_full()
                        .min_w_0()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(path.to_string()))
            .child(v_flex().w_full()
                           .min_w_0()
                           .children(diff_lines(old_text, new_text).into_iter()
                                                                   .map(|(color, text)| {
                                                                       div().w_full()
                                                                            .min_w_0()
                                                                            .font_family(style.mono_font_family.clone())
                                                                            .text_xs()
                                                                            .text_color(rgb(color))
                                                                            .child(text)
                                                                   })))
}

/// The `(color, text)` line list for a diff. With an `old_text` this is a
/// whole-file replacement, so every old line reads as removed and every
/// new line as added; without one the `new_text` is already in unified
/// form (or is a brand-new file) and its own `+`/`-` prefixes decide.
fn diff_lines(old_text: Option<&str>, new_text: &str) -> Vec<(u32, String)> {
    match old_text {
        Some(old) => old.lines()
                        .map(|line| (ERROR_COLOR, format!("- {line}")))
                        .chain(new_text.lines()
                                       .map(|line| (SAFE_COLOR, format!("+ {line}"))))
                        .collect(),
        None => new_text.lines()
                        .map(|line| match line.strip_prefix('+') {
                            Some(added) => (SAFE_COLOR, format!("+ {added}")),
                            None => match line.strip_prefix('-') {
                                Some(removed) => (ERROR_COLOR, format!("- {removed}")),
                                None => (MUTED, line.to_string()),
                            },
                        })
                        .collect(),
    }
}

/// Monospace tool output. `w_full`/`min_w_0` so a long line wraps inside
/// the card instead of stretching the whole conversation pane, per
/// `knot-ui-conventions.md`'s "Flex overflow" rule.
fn render_output_text(text: &str, style: &PanelStyle) -> impl IntoElement {
    div().w_full()
         .min_w_0()
         .font_family(style.mono_font_family.clone())
         .text_xs()
         .text_color(rgb(MUTED))
         .child(text.to_string())
}

/// An inline permission request with actionable allow/deny controls, per
/// `acp-panel-ui`'s permission-prompts requirement. Sending further
/// prompts is blocked by the caller while this is rendered (the caller
/// checks `PanelState::pending_permission` before calling `prompt`).
fn render_permission_prompt(request: &PermissionRequest, permission_risk: RiskLevel,
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
            "Permission requested for tool call {}",
            request.tool_call_id
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
mod tests {
    use super::*;

    #[test]
    fn every_known_kind_maps_to_a_distinct_icon_and_unknown_kinds_fall_back() {
        let known = ["read", "edit", "delete", "move", "search", "execute", "think", "fetch"];
        for kind in known {
            assert_ne!(tool_call_icon(kind),
                       IconName::Wrench,
                       "expected a specific icon for known kind {kind:?}");
        }
        assert_eq!(tool_call_icon("some-future-kind"), IconName::Wrench);
        assert_eq!(tool_call_icon(""), IconName::Wrench);
    }

    /// The bug this guards: keying the in-progress placeholder on "no
    /// output yet" left a completed call showing "Running…" forever,
    /// because a call can finish without ever reporting content.
    #[test]
    fn only_a_running_status_reads_as_running() {
        assert_eq!(status_label("in_progress"), "Running…");
        assert_eq!(status_label("pending"), "Pending");
        assert_eq!(status_label("completed"), "Done");
        assert_eq!(status_label("failed"), "Failed");
    }

    #[test]
    fn an_unrecognized_status_passes_through_rather_than_vanishing() {
        assert_eq!(status_label("some_future_status"), "some_future_status");
    }

    #[test]
    fn a_failed_call_is_the_only_status_outlined_in_danger() {
        assert_eq!(card_outline("failed"), CardOutline::Danger);
        assert_eq!(card_outline("completed"), CardOutline::Neutral);
        assert_eq!(card_outline("pending"), CardOutline::Info);
        assert_eq!(card_outline("in_progress"), CardOutline::Info);
    }

    /// Success is the common case: outlining every finished call in a
    /// colour of its own would leave the two states worth noticing - a
    /// call still running and one that failed - looking like the rest.
    #[test]
    fn a_completed_call_recedes_to_the_neutral_border() {
        assert_eq!(card_outline("completed"), CardOutline::Neutral);
    }

    /// `status` is a plain wire string, so a value this build has never
    /// heard of must render as an ordinary card rather than a failure -
    /// or panic.
    #[test]
    fn an_unrecognized_status_takes_the_neutral_border() {
        assert_eq!(card_outline("some_future_status"), CardOutline::Neutral);
        assert_eq!(card_outline(""), CardOutline::Neutral);
    }

    #[test]
    fn a_replacement_diff_reads_as_removals_then_additions() {
        let lines = diff_lines(Some("a\nb"), "a\nc");

        assert_eq!(lines,
                   vec![(ERROR_COLOR, "- a".to_string()),
                        (ERROR_COLOR, "- b".to_string()),
                        (SAFE_COLOR, "+ a".to_string()),
                        (SAFE_COLOR, "+ c".to_string())]);
    }

    #[test]
    fn a_new_file_diff_is_all_additions_with_no_prefixes_to_strip() {
        let lines = diff_lines(None, "fn main() {}");

        assert_eq!(lines, vec![(MUTED, "fn main() {}".to_string())]);
    }

    #[test]
    fn a_unified_diff_without_old_text_colors_by_prefix() {
        let lines = diff_lines(None, "-old\n+new\n context");

        assert_eq!(lines,
                   vec![(ERROR_COLOR, "- old".to_string()),
                        (SAFE_COLOR, "+ new".to_string()),
                        (MUTED, " context".to_string())]);
    }

    #[test]
    fn the_chevron_states_which_way_the_card_goes() {
        assert_eq!(disclosure_icon(true), IconName::ChevronRight);
        assert_eq!(disclosure_icon(false), IconName::ChevronDown);
    }

    /// Two cards in one conversation must not share a header element id,
    /// or a click on one would carry the other's state.
    #[test]
    fn each_tool_call_id_gets_its_own_element_id() {
        assert_eq!(element_id("tc1"), element_id("tc1"));
        assert_ne!(element_id("tc1"), element_id("tc2"));
    }

    #[test]
    fn permission_modes_are_classified_case_insensitively() {
        assert_eq!(permission_risk_level("bypassPermissions", "Restricted"),
                   RiskLevel::Danger);
        assert_eq!(permission_risk_level("unknown", "PLAN"), RiskLevel::Safe);
        assert_eq!(permission_risk_level("default", "Normal"),
                   RiskLevel::Neutral);
    }

    fn permission_request() -> PermissionRequest {
        PermissionRequest { rpc_id:       serde_json::json!(1),
                            tool_call_id: "tc1".to_string(),
                            options:      Vec::new(), }
    }

    /// The list's item count must cover the trailing permission and ended
    /// cards, or tail-following would stop short of them.
    #[test]
    fn row_count_covers_messages_and_the_trailing_cards() {
        let mut state = PanelState::new();
        assert_eq!(row_count(&state), 0);

        state.messages.push(PanelMessage::User { text: "hi".to_string(), queued: false });
        assert_eq!(row_count(&state), 1);

        state.pending_permission = Some(permission_request());
        assert_eq!(row_count(&state), 2);

        state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: Some(1) });
        assert_eq!(row_count(&state), 3);
    }

    /// Indices walk the messages in order, then the permission prompt, then
    /// the ended banner, and anything past the end resolves to nothing
    /// rather than panicking.
    #[test]
    fn row_at_walks_messages_then_permission_then_ended() {
        let mut state = PanelState::new();
        state.messages.push(PanelMessage::User { text: "a".to_string(), queued: false });
        state.messages
             .push(PanelMessage::Assistant("b".to_string()));
        state.pending_permission = Some(permission_request());
        state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: None });

        assert_eq!(row_at(&state, 0), Some(PanelRow::Message(0)));
        assert_eq!(row_at(&state, 1), Some(PanelRow::Message(1)));
        assert_eq!(row_at(&state, 2), Some(PanelRow::Permission));
        assert_eq!(row_at(&state, 3), Some(PanelRow::Ended));
        assert_eq!(row_at(&state, 4), None);
    }

    /// The ended row is the *last* row, not tied to a fixed index: with no
    /// permission pending it follows the messages directly.
    #[test]
    fn an_ended_row_follows_the_messages_when_no_permission_is_pending() {
        let mut state = PanelState::new();
        state.messages.push(PanelMessage::User { text: "a".to_string(), queued: false });
        state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: Some(0) });

        assert_eq!(row_count(&state), 2);
        assert_eq!(row_at(&state, 1), Some(PanelRow::Ended));
        assert_eq!(row_at(&state, 2), None);
    }

    /// `sync_row_count` is what keeps a live list's item count in step with
    /// the state, growing and shrinking by the delta so rows already on
    /// screen are left alone.
    #[test]
    fn sync_row_count_tracks_messages_and_trailing_rows() {
        let list = ListState::new(0, gpui_kit::ListAlignment::Top, px(LIST_OVERDRAW));
        let mut state = PanelState::new();
        let mut known = 0;

        known = sync_row_count(&list, known, &state);
        assert_eq!((list.item_count(), known), (0, 0));

        state.messages.push(PanelMessage::User { text: "a".to_string(), queued: false });
        state.messages
             .push(PanelMessage::Assistant("b".to_string()));
        known = sync_row_count(&list, known, &state);
        assert_eq!((list.item_count(), known), (2, 2));

        state.messages
             .push(PanelMessage::Assistant("c".to_string()));
        known = sync_row_count(&list, known, &state);
        assert_eq!((list.item_count(), known), (3, 3));

        state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: None });
        known = sync_row_count(&list, known, &state);
        assert_eq!((list.item_count(), known), (4, 4));

        state.messages.truncate(1);
        state.ended = None;
        known = sync_row_count(&list, known, &state);
        assert_eq!((list.item_count(), known), (1, 1));
    }

    /// The reconcile in `render_panel_pane` mirrors the track toggle onto
    /// these two calls: `Tail` while following, `Normal` otherwise. `Normal`
    /// is what keeps a toggled-off list put even at the tail, so it must not
    /// report as following afterwards.
    #[test]
    fn follow_mode_reflects_the_toggle() {
        let list = ListState::new(0, gpui_kit::ListAlignment::Top, px(LIST_OVERDRAW));
        assert!(!list.is_following_tail());

        list.set_follow_mode(gpui_kit::FollowMode::Tail);
        assert!(list.is_following_tail());

        list.set_follow_mode(gpui_kit::FollowMode::Normal);
        assert!(!list.is_following_tail());
    }
}
