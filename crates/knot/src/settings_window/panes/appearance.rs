use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::v_flex;

use crate::app_support::FontPanelTarget;
use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    pub(crate) fn render_appearance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            crate::controls::group(knot_core::l10n::t("settings.appearance.fonts"))
                .child(Self::row(
                    knot_core::l10n::t("settings.appearance.ui"),
                    Self::font_picker_button(
                        "ui-font-picker",
                        FontPanelTarget::Ui,
                        self.settings.ui_font_name.clone(),
                        self.settings.ui_font_size,
                        cx,
                    ),
                ))
                .child(Self::row(
                    knot_core::l10n::t("settings.appearance.title"),
                    Self::font_picker_button(
                        "title-font-picker",
                        FontPanelTarget::Title,
                        self.settings.title_font_name.clone(),
                        self.settings.title_font_size,
                        cx,
                    ),
                ))
                .child(Self::row(
                    knot_core::l10n::t("settings.appearance.terminal"),
                    Self::font_picker_button(
                        "terminal-font-picker",
                        FontPanelTarget::Terminal,
                        self.settings.terminal_font_name.clone(),
                        self.settings.terminal_font_size,
                        cx,
                    ),
                )),
        )
    }
}
