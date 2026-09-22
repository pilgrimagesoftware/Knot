use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Escape;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::input::TextareaState;

use crate::window_options::broadcast_window_options;

/// The broadcast message sheet: one multi-line field, Cancel and Send, per
/// `agent-list-ui`'s "Broadcast to All Agents" requirement.
///
/// A window of its own rather than an in-place overlay, the way the agent
/// editor is - every dialog in this app that takes more than a yes/no is a
/// window, and that is also what makes "starts empty each time" free: the
/// state is created with the window and dies with it, so there is no previous
/// message left anywhere to carry forward.
pub(crate) struct BroadcastSheet {
    message:       Entity<TextareaState>,
    on_send:       BroadcastHandler,
    _subscription: Subscription,
}

/// What the sheet does with the message once the user sends it - delivering
/// it is the caller's business, not the sheet's.
type BroadcastHandler = Box<dyn Fn(String, &mut Window, &mut App)>;

impl BroadcastSheet {
    /// The trimmed message, or `None` while there is nothing to send.
    fn trimmed(&self, cx: &App) -> Option<String> {
        let text = self.message.read(cx).value().trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    /// Hands the trimmed message to the caller and closes the sheet. A
    /// whitespace-only message does nothing, matching the disabled Send
    /// button rather than closing the sheet on an empty broadcast.
    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.trimmed(cx)
        else {
            return;
        };
        (self.on_send)(text, window, cx);
        window.remove_window();
    }
}

/// Opens the broadcast sheet, calling `on_send` with the trimmed message once
/// the user sends it.
pub(crate) fn open_broadcast_sheet(on_send: impl Fn(String, &mut Window, &mut App) + 'static,
                                   cx: &mut App) {
    let options = broadcast_window_options(cx);
    let _ =
        cx.open_window(options, move |window, cx| {
              let message = cx.new(|cx| {
                                  TextareaState::new(window, cx)
                        .placeholder(knot_core::l10n::t("broadcast.placeholder"))
                        .auto_grow(4, 16)
                              });
              cx.new(|cx| {
                    let subscription =
                        cx.subscribe_in(&message,
                                        window,
                                        |sheet: &mut BroadcastSheet, _, event, window, cx| {
                                            match event {
                                                // Modifier-Return sends;
                                                // plain Return inserts a
                                                // newline, since the field
                                                // is multi-line.
                                                InputEvent::PressEnter { secondary: true, .. } => {
                                                    sheet.send(window, cx);
                                                }
                                                // Send is disabled while
                                                // the message is empty, so
                                                // it has to re-render as
                                                // the user types rather
                                                // than on the next
                                                // unrelated repaint.
                                                InputEvent::Change => cx.notify(),
                                                _ => {}
                                            }
                                        });
                    message.update(cx, |state, cx| state.focus(window, cx));
                    BroadcastSheet { message,
                                     on_send: Box::new(on_send),
                                     _subscription: subscription }
                })
          });
}

impl Render for BroadcastSheet {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send = self.trimmed(cx).is_some();
        v_flex().size_full()
                .gap_3()
                .px_5()
                .pt_5()
                .pb_6()
                .bg(cx.theme().background)
                // Escape cancels, discarding the message. The textarea
                // propagates the action when it has nothing of its own to
                // dismiss, so this catches it from the focused field.
                .on_action(cx.listener(|_, _: &Escape, window, _| window.remove_window()))
                .child(Textarea::new(&self.message).flex_1().w_full())
                .child(h_flex().flex_shrink_0()
                               .justify_end()
                               .gap_2()
                               .child(Button::new("cancel-broadcast").label(knot_core::l10n::t("broadcast.cancel"))
                                                                     .on_click(|_, window, _| {
                                                                         window.remove_window()
                                                                     }))
                               .child(Button::new("send-broadcast").label(knot_core::l10n::t("broadcast.send"))
                                                                   .primary()
                                                                   .disabled(!can_send)
                                                                   .on_click(cx.listener(
                        |sheet, _, window, cx| sheet.send(window, cx),
                    ))))
                .children(crate::app_support::root_overlays(window, cx))
    }
}
