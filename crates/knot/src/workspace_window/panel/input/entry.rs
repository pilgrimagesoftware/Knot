//! The row the user types into: the add-context control, the composer
//! itself, and the send or stop button at its end.
//!
//! The composer's own wiring - the lookup keys and the paste interception -
//! is applied here, around the widget rather than inside it, so the widget
//! stays the one thing the rich-input change swaps.

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Paste;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::rgb;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInput;
use crate::workspace_window::panel::prompt::PanelInputState;

impl WorkspaceWindow {
    /// The attach / compose / send row for `id`.
    ///
    /// `can_send` and the send tooltip are derived here rather than passed
    /// in: both are functions of the buffer and the send-chord setting,
    /// which this row already has in hand.
    pub(super) fn render_panel_entry_row(&self, id: Uuid, input: &Entity<PanelInputState>,
                                         blocked: bool, turn_active: bool,
                                         cx: &mut Context<Self>)
                                         -> impl IntoElement + use<> {
        let can_send = !blocked && !turn_active && !input.read(cx).value().trim().is_empty();
        let send_tooltip = if crate::settings_global::read(cx).agent_panel_shift_enter_sends {
            "Send (Shift+Enter)"
        }
        else {
            "Send (Enter)"
        };
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
                    })
    }
}
