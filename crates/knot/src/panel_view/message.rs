//! One conversation message and the controls that hang off it: the
//! in-flight response's auto-scroll toggle, and a finished response's action
//! bar.

use std::rc::Rc;

use gpui_kit::ClickEvent;
use gpui_kit::ClipboardItem;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ListOffset;
use gpui_kit::ListState;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::notification::Notification;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::relative;
use gpui_kit::rgb;

use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;
use crate::panel_view::callbacks::PanelCallbacks;
use crate::panel_view::shell_card::render_shell_card;
use crate::panel_view::style::ERROR_COLOR;
use crate::panel_view::style::PanelStyle;
use crate::panel_view::tool_call::render_tool_call_card;

/// The hover group that reveals a prompt's copy control.
///
/// `group_hover` binds to the nearest ancestor carrying the name and each
/// prompt builds its own group, so one constant gives per-message behaviour -
/// keying it by index would allocate per message on the render path and
/// suggest the name is an identity when it is a lookup.
const PROMPT_HOVER_GROUP: &str = "panel-prompt";

/// One message's render inputs, grouped so `render_message` keeps a short
/// parameter list as the panel gains styling.
pub(super) struct Message<'a> {
    pub(super) state:   &'a PanelState,
    pub(super) index:   usize,
    pub(super) is_last: bool,
    pub(super) style:   &'a PanelStyle,
    pub(super) list:    &'a ListState,
}

pub(super) fn render_message(ctx: Message<'_>, message: &PanelMessage,
                             callbacks: &PanelCallbacks)
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
        PanelMessage::User(text) => {
            let copy_text = text.clone();
            h_flex()
                .w_full()
                .min_w_0()
                .justify_end()
                // The hover group is this shrink-wrapped cluster, not the
                // full-width row: a group on the row would reveal the control
                // from the empty space left of a short prompt, which
                // acp-panel-ui's "empty space beside a prompt reveals nothing"
                // rules out. The button being inside the group is also what
                // keeps it shown once the pointer reaches it.
                .child(
                    // The 85% cap belongs on the cluster, not on the bubble
                    // inside it. A percentage resolves against its containing
                    // block, and the cluster is shrink-wrapped, so a cap on
                    // the bubble resolved against an indefinite width and
                    // stopped constraining anything - a long prompt then grew
                    // past the row and read as left-aligned.
                    h_flex()
                        .max_w(relative(0.85))
                        .min_w_0()
                        .gap_1()
                        .group(PROMPT_HOVER_GROUP)
                        .child(
                            Button::new(("panel-copy-prompt", index as u64))
                                .icon(IconName::Copy)
                                .tooltip(knot_core::l10n::t("panel.copy_prompt"))
                                .ghost()
                                .small()
                                // `Visibility::Hidden` keeps the button in the
                                // layout, so revealing it cannot reflow the
                                // conversation as the pointer travels down it.
                                .invisible()
                                .group_hover(PROMPT_HOVER_GROUP, |style| style.visible())
                                .on_click(move |_: &ClickEvent, window, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copy_text.clone(),
                                    ));
                                    window.push_notification(
                                        Notification::info(knot_core::l10n::t(
                                            "panel.copied_prompt",
                                        )),
                                        cx,
                                    );
                                }),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .text_sm()
                                .text_color(style.prompt_foreground)
                                .px_3()
                                .py_1p5()
                                .rounded_md()
                                .bg(style.prompt_color)
                                .child(text.clone()),
                        ),
                )
                .into_any_element()
        }
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
                                .child(crate::markdown_view::markdown_view(
                        ("panel-message-markdown", index as u64),
                        text.clone(),
                        style.ui_font_family.clone(),
                        style.title_font_family.clone(),
                        style.markdown_font_size,
                    )))
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
        // The user's own command. Full width and on the card surface like a
        // tool call, but led by a `$` and the command itself, so it never
        // reads as something the agent did.
        PanelMessage::Shell(card) => {
            render_shell_card(card,
                              style,
                              callbacks.on_cancel_shell.clone(),
                              callbacks.on_discard_shell.clone()).into_any_element()
        }
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
pub(super) fn preceding_user_message(state: &PanelState, index: usize) -> Option<usize> {
    state.messages[..index].iter()
                           .rposition(|message| matches!(message, PanelMessage::User(_)))
}

/// The in-flight response's auto-scroll toggle, per the track toggle
/// design's per-response scope - shown only on the currently streaming
/// response, replaced by the response action bar once it finalizes.
pub(super) fn render_track_toggle(tracking: bool, on_toggle: Rc<dyn Fn()>) -> impl IntoElement {
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
pub(super) fn render_response_actions(text: String, user_index: Option<usize>, index: usize,
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
                .tooltip(knot_core::l10n::t("panel.copy_response"))
                .ghost()
                .small()
                .on_click(move |_: &ClickEvent, window, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                    window.push_notification(
                        Notification::info(knot_core::l10n::t("panel.copied_response")),
                        cx,
                    );
                }),
        )
        .children(user_index.map(|user_index| {
            Button::new(("panel-scroll-to-user", index as u64))
                .icon(IconName::ArrowUp)
                .tooltip(knot_core::l10n::t("panel.scroll_to_your_message"))
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
                .tooltip(knot_core::l10n::t("panel.scroll_to_top"))
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
