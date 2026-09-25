//! The list of prompts waiting above the composer, one row each.
//!
//! A row is offered for the agent, not by it: the status icon reports why
//! the entry is still waiting, and retry, edit and delete are the ways out
//! of that state.

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::element_key;
use crate::workspace_window::prompt_queue;
use crate::workspace_window::prompt_queue::QueuedPanelPrompt;
use crate::workspace_window::prompt_queue::queued_status_label;

impl WorkspaceWindow {
    /// The queued-prompt rows for `id`, or nothing when the queue is empty.
    pub(super) fn render_panel_queued_prompts(id: Uuid, queued_prompts: &[QueuedPanelPrompt],
                                              cx: &mut Context<Self>)
                                              -> Option<impl IntoElement + use<>> {
        (!queued_prompts.is_empty()).then(|| {
                                        v_flex().gap_1().children(queued_prompts.iter().map(
                |prompt| {
                    // Every row action names its entry by id: a
                    // delivery landing between this frame and the
                    // click shifts every index behind it, and two
                    // prompts reading the same text are ordinary. The
                    // element ids go by id for the same reason - GPUI
                    // keys hover and press state off them, so an
                    // index-keyed row inherits the state of whatever
                    // row held its position last frame.
                    let prompt_id = prompt.id;
                    h_flex()
                            .w_full()
                            .min_w_0()
                            .gap_1()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_xs()
                                    .font_family(cx.theme().mono_font_family.clone())
                                    // The row is flattened, `prompt.text`
                                    // is not: a prompt written across
                                    // several lines waits as one row, and
                                    // the agent still receives it as the
                                    // user wrote it.
                                    .child(single_line(&prompt.text)),
                            )
                            .child(
                                Button::new(("panel-queued-prompt-status", element_key(prompt_id)))
                                    .child(Icon::new(if prompt.failed {
                                        gpui_kit::assets::IconName::CircleX
                                    } else {
                                        gpui_kit::assets::IconName::Clock4
                                    }))
                                    .tooltip(queued_status_label(prompt.failed, prompt.origin))
                                    .accessibility_label(queued_status_label(prompt.failed,
                                                                             prompt.origin))
                                    .text_color(if prompt.failed {
                                        cx.theme().danger
                                    } else {
                                        cx.theme().muted_foreground
                                    })
                                    .ghost()
                                    .xsmall(),
                            )
                            // Retry and delete are separate controls, so a
                            // failed prompt can be dismissed rather than
                            // only re-sent - one row action cannot be both.
                            .children(prompt.failed.then(|| {
                                Button::new(("panel-queued-prompt-retry", element_key(prompt_id)))
                                    .icon(IconName::RotateCw)
                                    .tooltip(knot_core::l10n::t("panel.retry"))
                                    .accessibility_label(knot_core::l10n::t("panel.retry_queued"))
                                    // Not tinted red: the failure is the
                                    // state, retry is the way out of it,
                                    // and red is reserved for the
                                    // destructive control beside it.
                                    .text_color(cx.theme().muted_foreground)
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        if let Some(queue) = view.panel_prompt_queues.get_mut(&id)
                                            && prompt_queue::retry(queue, prompt_id)
                                        {
                                            cx.notify();
                                        }
                                    }))
                            }))
                            // Editing returns the message to the composer,
                            // so it is offered exactly where deletion is:
                            // a prompt the agent already has is neither.
                            .children(prompt.is_deletable().then(|| {
                                Button::new(("panel-queued-prompt-edit", element_key(prompt_id)))
                                    .icon(gpui_kit::assets::IconName::Pencil)
                                    .tooltip(knot_core::l10n::t("panel.edit_queued"))
                                    .accessibility_label(knot_core::l10n::t("panel.edit_queued"))
                                    .text_color(cx.theme().muted_foreground)
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |view,
                                                          _: &ClickEvent,
                                                          window,
                                                          cx| {
                                        view.edit_queued_prompt(id, prompt_id, window, cx);
                                    }))
                            }))
                            .children(prompt.is_deletable().then(|| {
                                Button::new(("panel-queued-prompt-delete", element_key(prompt_id)))
                                    .icon(gpui_kit::assets::IconName::Trash)
                                    .tooltip(knot_core::l10n::t("panel.delete_queued"))
                                    .accessibility_label(knot_core::l10n::t("panel.delete_queued"))
                                    // Red icon on a ghost button, per the
                                    // project's destructive-action
                                    // convention - `.danger()` would
                                    // replace `.ghost()` outright.
                                    .text_color(cx.theme().danger)
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        if let Some(queue) = view.panel_prompt_queues.get_mut(&id)
                                            && prompt_queue::remove(queue, prompt_id)
                                        {
                                            cx.notify();
                                        }
                                    }))
                            }))
                },
            ))
                                    })
    }
}
