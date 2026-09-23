//! The row under the composer: the context-usage indicator and send hint on
//! one side, the three session-config selectors and the expand control on
//! the other.
//!
//! The selectors are live controls over the agent's own Session Config
//! Options, so what they offer is whatever the adapter declared - which is
//! why an axis the agent never reported is drawn disabled rather than
//! omitted.

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::popover::Popover;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::rgb;
use uuid::Uuid;

use crate::panel_session;
use crate::panel_view;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::context_usage_indicator;
use crate::workspace_window::panel::input::PERMISSION_SELECTOR_ID;

impl WorkspaceWindow {
    /// The control bar under `id`'s composer.
    pub(super) fn render_panel_control_bar(&self, id: Uuid, expanded: bool,
                                           config_options: &[knot_acp::ConfigOption],
                                           cx: &mut Context<Self>)
                                           -> impl IntoElement + use<> {
        let shift_to_send = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
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
                                    .font_family(crate::settings_global::read(cx).title_font_name.clone())
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
                                entity.update(app, move |view, cx| {
                                    view.open_config_selector = None;
                                    // Persist first: the selection is durable
                                    // whether or not a session is live to
                                    // apply it to.
                                    view.remember_session_config(id,
                                                                 config_id.clone(),
                                                                 value_id.clone(),
                                                                 cx);
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
