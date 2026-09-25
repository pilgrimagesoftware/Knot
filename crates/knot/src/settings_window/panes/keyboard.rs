use gpui_kit::AnyElement;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Selectable;
use gpui_kit::component::button::Button;
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
        let mut group = crate::controls::group(knot_core::l10n::t("settings.keyboard.shortcuts"));
        for shortcut in Shortcut::ALL {
            group = group.child(Self::row(shortcut.label(),
                                          self.keybinding_control(shortcut, &resolved, cx)));
            if let Some((rejected, message)) = &self.keyboard.rejection
               && *rejected == shortcut
            {
                group = group.child(h_flex().gap_3()
                                            .child(div().w(gpui_kit::px(Self::LABEL_WIDTH))
                                                        .flex_shrink_0())
                                            .child(div().text_sm()
                                                        .text_color(cx.theme().danger)
                                                        .child(message.clone())));
            }
        }
        let settings_window = cx.entity();
        v_flex().gap_3()
                .child(div().text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("settings.keyboard.blurb")))
                .child(group)
                .child(h_flex().justify_end().child(
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
        let mut row = h_flex().gap_1().items_center();
        if let Some(modifiers) = resolved.modifiers(shortcut) {
            for (index, (glyph, is_on, toggle)) in FAMILY_MODIFIERS.into_iter().enumerate() {
                let settings_window = settings_window.clone();
                row = row.child(
                    Button::new(("keyboard-modifier", shortcut as usize * 4 + index))
                        .label(glyph)
                        .selected(is_on(&modifiers))
                        .on_click(move |_, _, app| {
                            settings_window.update(app, |view, cx| {
                                               view.toggle_family_modifier(shortcut, toggle, cx);
                                           });
                        }),
                );
            }
            row = row.child(div().text_sm()
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
            row = row.child(Button::new(("keyboard-record", shortcut as usize))
                                .label(label)
                                .selected(recording)
                                .on_click(move |_, _, app| {
                                    settings_window.update(app, |view, cx| {
                                                       view.start_recording(shortcut, cx);
                                                   });
                                }));
        }
        row.child(Button::new(("keyboard-reset", shortcut as usize))
                      .label(knot_core::l10n::t("settings.keyboard.reset"))
                      .on_click(move |_, _, app| {
                          settings_window.update(app, |view, cx| view.reset_keybinding(shortcut, cx));
                      }))
           .into_any_element()
    }
}
