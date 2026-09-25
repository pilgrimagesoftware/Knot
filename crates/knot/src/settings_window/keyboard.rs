//! The Keyboard tab's state and the edits it makes (`keybindings`).
//!
//! Every edit builds a candidate [`Resolved`] from the one in effect,
//! validates it for the shortcut that changed, and either commits it -
//! preference, keymap and menu bar together - or keeps the rejection for
//! the pane to show. Nothing is written for a rejected edit.

use gpui_kit::Context;
use gpui_kit::Keystroke;
use gpui_kit::Subscription;
use knot_core::ShortcutModifiers;

use crate::keymap::Chord;
use crate::keymap::Resolved;
use crate::keymap::Shortcut;
use crate::settings_window::SettingsWindow;

/// Held by the settings window across tab switches, like every pane's
/// in-progress state (`settings-ui`: "Tab switching preserves window
/// state").
#[derive(Default)]
pub(crate) struct KeyboardPaneState {
    /// The shortcut whose recorder is armed.
    pub(crate) recording:    Option<Shortcut>,
    /// The last refused edit and why, shown under its row until the next
    /// edit.
    pub(crate) rejection:    Option<(Shortcut, String)>,
    /// Swallows keystrokes while a recorder is armed; dropping it disarms.
    pub(crate) _interceptor: Option<Subscription>,
}

/// gpui reports a lone modifier press as a keystroke named after the
/// modifier. A recorder waits past those for the key they modify.
fn is_modifier_only(keystroke: &Keystroke) -> bool {
    matches!(keystroke.key.as_str(),
             "shift" | "control" | "alt" | "platform" | "function")
}

impl SettingsWindow {
    /// The pane's recorder and rejection, for the tests that assert on them.
    #[cfg(test)]
    pub(crate) fn keyboard_state(&self) -> &KeyboardPaneState {
        &self.keyboard
    }

    /// The configurable shortcuts as currently in effect.
    pub(crate) fn resolved_keybindings(cx: &gpui_kit::App) -> Resolved {
        Resolved::from_settings(&crate::settings_global::read(cx).keybindings)
    }

    /// Arms `shortcut`'s recorder. The next non-modifier keystroke anywhere
    /// in the app is taken as its chord - and stopped, so a chord that is
    /// already bound records rather than runs. Escape cancels.
    pub(crate) fn start_recording(&mut self, shortcut: Shortcut, cx: &mut Context<Self>) {
        let this = cx.entity().downgrade();
        let interceptor = cx.intercept_keystrokes(move |event, _window, app| {
                                if is_modifier_only(&event.keystroke) {
                                    return;
                                }
                                app.stop_propagation();
                                let keystroke = event.keystroke.clone();
                                let _ = this.update(app, |view, cx| {
                                                view.finish_recording(&keystroke, cx);
                                            });
                            });
        self.keyboard = KeyboardPaneState { recording:    Some(shortcut),
                                            rejection:    None,
                                            _interceptor: Some(interceptor), };
        cx.notify();
    }

    pub(crate) fn finish_recording(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        let Some(shortcut) = self.keyboard.recording.take()
        else {
            return;
        };
        self.keyboard._interceptor = None;
        let unmodified = !keystroke.modifiers.modified();
        if unmodified && keystroke.key == "escape" {
            cx.notify();
            return;
        }
        let chord = Chord::from_keystroke(keystroke);
        let candidate = Self::resolved_keybindings(cx).with_chord(shortcut, chord);
        self.commit_keybindings(shortcut, candidate, cx);
    }

    pub(crate) fn toggle_family_modifier(&mut self, shortcut: Shortcut,
                                         toggle: fn(&mut ShortcutModifiers),
                                         cx: &mut Context<Self>) {
        let current = Self::resolved_keybindings(cx);
        let Some(mut modifiers) = current.modifiers(shortcut)
        else {
            return;
        };
        toggle(&mut modifiers);
        self.commit_keybindings(shortcut, current.with_modifiers(shortcut, modifiers), cx);
    }

    pub(crate) fn reset_keybinding(&mut self, shortcut: Shortcut, cx: &mut Context<Self>) {
        let candidate = Self::resolved_keybindings(cx).reset(shortcut);
        self.commit_keybindings(shortcut, candidate, cx);
    }

    /// Restores every default. Needs no validation: the defaults are
    /// checked against each other and the fixed set by the keymap tests.
    pub(crate) fn restore_default_keybindings(&mut self, cx: &mut Context<Self>) {
        self.keyboard = KeyboardPaneState::default();
        self.install_keybindings(&Resolved::defaults(), cx);
    }

    fn commit_keybindings(&mut self, shortcut: Shortcut, candidate: Resolved,
                          cx: &mut Context<Self>) {
        match crate::keymap::validate(&candidate, shortcut) {
            Ok(()) => {
                self.keyboard.rejection = None;
                self.install_keybindings(&candidate, cx);
            }
            Err(rejection) => {
                self.keyboard.rejection = Some((shortcut, rejection.message()));
                cx.notify();
            }
        }
    }

    fn install_keybindings(&mut self, resolved: &Resolved, cx: &mut Context<Self>) {
        let stored = resolved.to_settings();
        crate::settings_global::write(cx, |settings| settings.keybindings = stored.clone());
        self.persist(cx);
        crate::keymap::apply_and_refresh_menus(resolved, cx);
        cx.notify();
    }
}
