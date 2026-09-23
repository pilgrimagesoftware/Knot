//! The panel's composer: the prompt textarea and everything stacked around
//! it - queued prompts, attached context chips, the send/stop control, and
//! the three session config selectors.

use std::path::PathBuf;

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Paste;
use gpui_kit::component::popover::Popover;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;
use gpui_kit::rgb;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::panel_session;
use crate::panel_view;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::context_usage_indicator;
use crate::workspace_window::element_key;
use crate::workspace_window::panel::prompt::PanelInput;
use crate::workspace_window::panel::prompt::PanelInputState;
use crate::workspace_window::prompt_queue;
use crate::workspace_window::prompt_queue::QueuedPanelPrompt;
use crate::workspace_window::prompt_queue::queued_status_label;

/// The permission-mode selector's element id - the one selector with
/// risk-tinted labels and a keyboard action of its own, so it needs naming
/// rather than matching on a literal in three places.
pub(in crate::workspace_window) const PERMISSION_SELECTOR_ID: &str =
    "panel-permission-mode-selector";

impl WorkspaceWindow {
    /// The input area: attached-context chips, the expandable text entry,
    /// and a control row (add-context, permission mode, model, effort,
    /// expand/collapse, send) - a sibling of the message list under
    /// `render_panel_pane`, per design decision "Control bar placement".
    #[allow(clippy::too_many_arguments)]
    pub(in crate::workspace_window) fn render_panel_input_area(&mut self, id: Uuid,
                                                               input: &Entity<PanelInputState>,
                                                               pending_context: &[PathBuf],
                                                               queued_prompts: &[QueuedPanelPrompt],
                                                               expanded: bool, blocked: bool,
                                                               turn_active: bool,
                                                               config_options: &[knot_acp::ConfigOption],
                                                               cx: &mut Context<Self>)
                                                               -> impl IntoElement {
        let can_send = !blocked && !turn_active && !input.read(cx).value().trim().is_empty();
        if !turn_active {
            self.panel_stopping.remove(&id);
        }
        let shift_to_send = self.settings.agent_panel_shift_enter_sends;
        let send_tooltip = if shift_to_send {
            "Send (Shift+Enter)"
        }
        else {
            "Send (Enter)"
        };
        let context_usage =
            self.panel_sessions.get(&id).and_then(|slot| {
                                            let slot = slot.lock();
                                            let panel_session::PanelSessionSlot::Ready(handle) =
                                                &*slot
                                            else {
                                                return None;
                                            };
                                            let state = handle.state();
                                            state.lock().context_usage
                                        });
        // Built before the element chain below, which borrows `self`
        // immutably: the lookup's registry is memoized per agent, so
        // producing the popup needs `&mut self`.
        let lookup = self.render_panel_lookup(id, input, cx);
        v_flex()
            .flex_shrink_0()
            .gap_2()
            .p_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .children((!queued_prompts.is_empty()).then(|| {
                v_flex()
                    .gap_1()
                    .children(queued_prompts.iter().map(|prompt| {
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
                    }))
            }))
            // Files and images dragged from Finder attach the same way the
            // paperclip and a pasted screenshot do, per `acp-panel-ui`'s
            // attached-context requirement.
            .drag_over::<gpui_kit::ExternalPaths>(|style, _, _, app| style.bg(app.theme().accent))
            .on_drop(
                cx.listener(move |view, paths: &gpui_kit::ExternalPaths, _, cx| {
                    view.panel_pending_context
                        .entry(id)
                        .or_default()
                        .extend(paths.paths().iter().cloned());
                    cx.notify();
                }),
            )
            .children((!pending_context.is_empty()).then(|| {
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(pending_context.iter().enumerate().map(|(index, path)| {
                        let file_name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.to_string_lossy().into_owned());
                        h_flex()
                            .gap_1()
                            .items_center()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(cx.theme().secondary)
                            .child(div().text_xs().child(file_name))
                            .child(
                                Button::new(("panel-remove-context", index as u64))
                                    .icon(IconName::CircleX)
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        view.remove_panel_context(id, index);
                                        cx.notify();
                                    })),
                            )
                    }))
            }))
            // The slash lookup sits directly above the prompt row: inside
            // the input area, so the conversation's scroll container cannot
            // clip it, and above the caret rather than over the line being
            // typed.
            .children(lookup)
            // Attach, prompt, and Send share one row (`items_center`, so
            // the two buttons sit centred against the prompt box however
            // tall it is); the send hint shares the row below with the
            // config selectors, pushed apart by `justify_between`.
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("panel-add-context")
                            .icon(gpui_kit::component::Icon::new(
                                gpui_kit::assets::IconName::Paperclip,
                            ))
                            .tooltip(knot_core::l10n::t("panel.attach_context"))
                            .ghost()
                            .small()
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.add_panel_context(id, cx);
                            })),
                    )
                    .child(
                        self.wire_panel_lookup_keys(div()
                            .flex_1()
                            .min_w_0(), id, input, cx)
                            .capture_action::<Paste>({
                                let entity = cx.entity();
                                move |_, _, app| {
                                    entity.update(app, |view, cx| {
                                        if view.paste_clipboard_image_context(id, cx) {
                                            cx.stop_propagation();
                                            cx.notify();
                                        }
                                    });
                                }
                            })
                            .child(PanelInput::new(input).w_full().disabled(blocked)),
                    )
                    .child(if turn_active {
                        Button::new("panel-stop-prompt")
                            .child(div().size(px(10.)).rounded(px(1.)).bg(rgb(0xFFFFFF)))
                            .tooltip(knot_core::l10n::t("panel.stop"))
                            .bg(rgb(0xEF4444))
                            .text_color(rgb(0xFFFFFF))
                            .flex_shrink_0()
                            .disabled(self.panel_stopping.contains(&id))
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.stop_panel_prompt(id, cx);
                            }))
                            .into_any_element()
                    } else {
                        Button::new("panel-send-prompt")
                            .icon(gpui_kit::assets::IconName::Send)
                            .tooltip(send_tooltip)
                            .primary()
                            .flex_shrink_0()
                            .disabled(!can_send)
                            .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                view.send_panel_prompt(id, window, cx);
                            }))
                            .into_any_element()
                    }),
            )
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .children(context_usage.map(|(used, size)| {
                                context_usage_indicator(used, size, cx)
                            }))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_xs()
                                    .font_family(self.settings.title_font_name.clone())
                                    .text_color(cx.theme().muted_foreground)
                                    .child(Self::panel_prompt_send_hint(shift_to_send)),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(self.render_panel_config_selector(
                                id,
                                PERMISSION_SELECTOR_ID,
                                "Permission",
                                "This agent doesn't report permission modes",
                                Self::find_config_option(
                                    config_options,
                                    &["mode", "permission_mode", "permission-mode"],
                                ),
                                cx,
                            ))
                            .child(self.render_panel_config_selector(
                                id,
                                "panel-model-selector",
                                "Model",
                                "This agent doesn't report selectable models",
                                Self::find_config_option(config_options, &["model"]),
                                cx,
                            ))
                            .child(self.render_panel_config_selector(
                                id,
                                "panel-effort-selector",
                                "Effort",
                                "This agent doesn't report selectable effort levels",
                                Self::find_config_option(
                                    config_options,
                                    &[
                                        "effort",
                                        "reasoning",
                                        "reasoning_effort",
                                        "reasoning-effort",
                                        "thought_level",
                                        "thought-level",
                                    ],
                                ),
                                cx,
                            ))
                            .child(
                                Button::new("panel-expand-input")
                                    .icon(if expanded {
                                        IconName::Minimize
                                    } else {
                                        IconName::Maximize
                                    })
                                    .tooltip(if expanded { "Collapse" } else { "Expand" })
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        view.toggle_panel_input_expanded(id, cx);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }

    /// One of the input area's three selector slots (permission mode,
    /// model, effort), sourced from the agent's Session Config Options -
    /// per ACP's stabilized mechanism, a live control that applies the
    /// selection via `session/set_config_option`. Disabled with an
    /// explanatory tooltip when the agent hasn't declared a matching
    /// option (adapters vary in which axes they expose).
    pub(in crate::workspace_window) fn render_panel_config_selector(&self, id: Uuid,
                                                                    element_id: &'static str,
                                                                    placeholder: &'static str,
                                                                    disabled_tooltip: &'static str,
                                                                    option: Option<&knot_acp::ConfigOption>,
                                                                    cx: &mut Context<Self>)
                                                                    -> gpui_kit::AnyElement {
        let Some(option) = option
        else {
            return Button::new(element_id).label(placeholder)
                                          .tooltip(disabled_tooltip)
                                          .ghost()
                                          .small()
                                          .disabled(true)
                                          .into_any_element();
        };
        let current_value = option.current_value.as_str().unwrap_or_default();
        let current_label = option.options
                                  .iter()
                                  .find(|value| value.value == current_value)
                                  .map(|value| value.name.clone())
                                  .unwrap_or_else(|| option.name.clone());
        let config_id = option.id.clone();
        let values = option.options.clone();
        let entity = cx.entity();
        let session_arc = self.panel_sessions.get(&id).cloned();
        let is_permission_selector = element_id == PERMISSION_SELECTOR_ID;
        let selector_color = is_permission_selector.then(|| {
                                 panel_view::risk_color(panel_view::permission_risk_level(
                    current_value,
                    &option.name,
                ))
                             })
                             .flatten();
        let trigger = Button::new(element_id).label(current_label)
                                             .ghost()
                                             .small()
                                             .dropdown_caret(true)
                                             .when_some(selector_color, |button, color| {
                                                 button.text_color(rgb(color))
                                             });
        Popover::new(format!("{element_id}-{id}")).anchor(gpui_kit::Anchor::BottomLeft)
                                                  .trigger(trigger)
                                                  .open(self.open_config_selector
                                                        == Some(element_id))
                                                  .on_open_change({
                                                      let entity = entity.clone();
                                                      move |open, _, app| {
                                                          entity.update(app, |view, cx| {
                                                                    view.open_config_selector =
                                                                        open.then_some(element_id);
                                                                    cx.notify();
                                                                });
                                                      }
                                                  })
                                                  .content(move |_, _window, _app| {
                                                      v_flex()
                    .gap_1()
                    .p_1()
                    .children(values.iter().enumerate().map(|(index, value)| {
                        let entity = entity.clone();
                        let session_arc = session_arc.clone();
                        let config_id = config_id.clone();
                        let value_id = value.value.clone();
                        let item_color = is_permission_selector
                            .then(|| {
                                panel_view::risk_color(panel_view::permission_risk_level(
                                    &value.value,
                                    &value.name,
                                ))
                            })
                            .flatten();
                        Button::new((element_id, index))
                            .accessibility_label(value.name.clone())
                            .child(div().w_full().child(value.name.clone()))
                            .ghost()
                            .small()
                            .w_full()
                            .when_some(item_color, |button, color| button.text_color(rgb(color)))
                            .on_click(move |_, _, app| {
                                let Some(session_arc) = session_arc.clone() else {
                                    return;
                                };
                                let config_id = config_id.clone();
                                let value_id = value_id.clone();
                                entity.update(app, move |view, _cx| {
                                    view.open_config_selector = None;
                                    // Persist first: the selection is durable
                                    // whether or not a session is live to
                                    // apply it to.
                                    view.remember_session_config(id,
                                                                 config_id.clone(),
                                                                 value_id.clone());
                                    if let panel_session::PanelSessionSlot::Ready(handle) =
                                        &*session_arc.lock()
                                    {
                                        let future = handle.set_config_option(config_id, value_id);
                                        let _guard = view.runtime.enter();
                                        view.runtime.spawn(future);
                                    }
                                });
                            })
                    }))
                                                  })
                                                  .into_any_element()
    }
}
