use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::Disableable;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::switch::Switch;
use gpui_kit::div;

use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    /// Display name for a modifier key code, ported from the Swift
    /// reference's `ModifierKeyCode.name(for:)`.
    pub(crate) fn key_name_for_code(code: i32) -> String {
        match code {
            54 => "Right Command",
            55 => "Left Command",
            56 => "Left Shift",
            57 => "Caps Lock",
            58 => "Left Option",
            59 => "Left Control",
            60 => "Right Shift",
            61 => "Right Option",
            62 => "Right Control",
            63 => "Fn",
            _ => return format!("Key {code}"),
        }.to_string()
    }

    pub(crate) fn render_voice(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let voice_enabled = self.settings.voice_enabled;
        let voice_auto_insert = self.settings.voice_auto_insert;
        let key_name = Self::key_name_for_code(self.settings.voice_push_to_talk_key);

        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        knot_core::l10n::t("settings.voice.blurb"),
                    ),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.voice.engine"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.voice.enable"),
                        Switch::new("voice-enabled")
                            .checked(voice_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        view.settings.voice_enabled = checked;
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.voice.engine"),
                        Button::new("voice-engine-picker")
                            .label(knot_core::l10n::t("settings.voice.engine_apple"))
                            .disabled(true),
                    ))
                    .child(Self::hint(
                        cx,
                        knot_core::l10n::t("settings.voice.engine_hint"),
                    )),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.voice.input"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.voice.push_to_talk"),
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .opacity(if voice_enabled { 1.0 } else { 0.5 })
                            .child(key_name),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.voice.auto_insert"),
                        Switch::new("voice-auto-insert")
                            .checked(voice_auto_insert)
                            .disabled(!voice_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        view.settings.voice_auto_insert = checked;
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        if voice_auto_insert {
                            "Transcribed text will be automatically inserted into the terminal."
                        } else {
                            "Transcribed text will be shown in a popup for review before \
                             insertion."
                        },
                    )),
            )
    }
}
