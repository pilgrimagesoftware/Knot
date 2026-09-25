use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Selectable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use knot_core::ShortcutModifiers;

use crate::keymap::Resolved;
use crate::keymap::Shortcut;
use crate::keymap::modifiers_label;
use crate::settings_window::SettingsWindow;

/// One modifier toggle: its glyph, whether it is held, and how to flip it.
type ModifierToggle = (&'static str, fn(&ShortcutModifiers) -> bool, fn(&mut ShortcutModifiers));

/// The four modifier toggles a numbered family offers, in macOS order.
const FAMILY_MODIFIERS: [ModifierToggle; 4] = [("⌃", |m| m.control, |m| m.control = !m.control),
                                               ("⌥", |m| m.alt, |m| m.alt = !m.alt),
                                               ("⇧", |m| m.shift, |m| m.shift = !m.shift),
                                               ("⌘", |m| m.command, |m| m.command = !m.command)];

impl SettingsWindow {
    pub(crate) fn render_keyboard(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let resolved = Self::resolved_keybindings(cx);
        let mut rows = v_flex().gap_3();
        for shortcut in Shortcut::ALL {
            rows = rows.child(Self::row(shortcut.label(),
                                        self.keybinding_control(shortcut, &resolved, cx)));
            if let Some((rejected, message)) = &self.keyboard.rejection
               && *rejected == shortcut
            {
                rows = rows.child(h_flex().gap_3()
                                          .child(div().w(gpui_kit::px(Self::LABEL_WIDTH))
                                                      .flex_shrink_0())
                                          .child(div().text_sm()
                                                      .text_color(cx.theme().danger)
                                                      .child(message.clone())));
            }
        }
        // Only the list scrolls: the blurb above and Restore Defaults below
        // stay where they are, like the Personas tab's title and actions.
        let list = div().id("keyboard-shortcuts-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .child(rows);
        let settings_window = cx.entity();
        v_flex().size_full()
                .gap_3()
                .child(div().flex_shrink_0()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("settings.keyboard.blurb")))
                .child(crate::controls::group(knot_core::l10n::t("settings.keyboard.shortcuts"))
                           .flex_1()
                           .min_h_0()
                           .child(list))
                .child(h_flex().flex_shrink_0().justify_end().child(
                    Button::new("keyboard-restore-defaults")
                        .label(knot_core::l10n::t("settings.keyboard.restore_defaults"))
                        .on_click(move |_, _, app| {
                            settings_window.update(app, |view, cx| view.restore_default_keybindings(cx));
                        }),
                ))
    }

    /// The control column of one shortcut's row: the modifier toggles for
    /// a numbered family, the recorder for a single chord, then Reset.
    fn keybinding_control(&self, shortcut: Shortcut, resolved: &Resolved, cx: &mut Context<Self>)
                          -> AnyElement {
        let settings_window = cx.entity();
        let mut binding = h_flex().gap_1().items_center();
        if let Some(modifiers) = resolved.modifiers(shortcut) {
            for (index, (glyph, is_on, toggle)) in FAMILY_MODIFIERS.into_iter().enumerate() {
                let settings_window = settings_window.clone();
                // A held modifier draws in the primary variant, which carries
                // the user's system accent color (`app_support`); the
                // selected state was too faint to read at a glance.
                let button =
                    Button::new(("keyboard-modifier", shortcut as usize * 4 + index)).label(glyph);
                let button = if is_on(&modifiers) {
                    button.primary()
                }
                else {
                    button
                };
                binding = binding.child(button.on_click(move |_, _, app| {
                                                  settings_window.update(app, |view, cx| {
                                                      view.toggle_family_modifier(shortcut,
                                                                                  toggle, cx);
                                                  });
                                              }));
            }
            binding = binding.child(div().text_sm()
                                         .text_color(cx.theme().muted_foreground)
                                         .child(format!("{}1…{}9",
                                                        modifiers_label(modifiers),
                                                        modifiers_label(modifiers))));
        }
        else if let Some(chord) = resolved.chord(shortcut) {
            let recording = self.keyboard.recording == Some(shortcut);
            let label = if recording {
                knot_core::l10n::t("settings.keyboard.press_shortcut")
            }
            else {
                chord.label()
            };
            let settings_window = settings_window.clone();
            binding = binding.child(Button::new(("keyboard-record", shortcut as usize))
                                .label(label)
                                .selected(recording)
                                .on_click(move |_, _, app| {
                                    settings_window.update(app, |view, cx| {
                                                       view.start_recording(shortcut, cx);
                                                   });
                                }));
        }
        // Right-aligned, so every row's Reset lines up whatever width its
        // binding controls take.
        let reset = crate::controls::icon_button(("keyboard-reset", shortcut as usize),
                                                 "icons/rotate-ccw.svg",
                                                 knot_core::l10n::t("settings.keyboard.reset"),
                                                 false).on_click(move |_, _, app| {
                                                           settings_window.update(app, |view, cx| {
                                                               view.reset_keybinding(shortcut, cx)
                                                           });
                                                       });
        h_flex().flex_1()
                .min_w_0()
                .justify_between()
                .items_center()
                .child(binding)
                .child(reset)
                .into_any_element()
    }
}
