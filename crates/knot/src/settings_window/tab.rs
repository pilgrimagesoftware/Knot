//! The settings window's tabs - which panes exist, and what each is called.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Coding,
    Personas,
    Autopilot,
    Voice,
    Mcp,
    Terminal,
    Keyboard,
}

impl SettingsTab {
    pub(crate) const ALL: [SettingsTab; 8] = [SettingsTab::General,
                                              SettingsTab::Coding,
                                              SettingsTab::Personas,
                                              SettingsTab::Autopilot,
                                              SettingsTab::Voice,
                                              SettingsTab::Mcp,
                                              SettingsTab::Terminal,
                                              SettingsTab::Keyboard];

    /// The tab's title. `Terminal` is titled "Appearance": the pane grew
    /// from terminal appearance into the window's look as a whole, and the
    /// title followed while the variant did not.
    pub(crate) fn label(self) -> String {
        match self {
            SettingsTab::General => knot_core::l10n::t("settings.tabs.general"),
            SettingsTab::Coding => knot_core::l10n::t("settings.tabs.coding"),
            SettingsTab::Personas => knot_core::l10n::t("settings.tabs.personas"),
            SettingsTab::Autopilot => knot_core::l10n::t("settings.tabs.autopilot"),
            SettingsTab::Voice => knot_core::l10n::t("settings.tabs.voice"),
            SettingsTab::Mcp => knot_core::l10n::t("settings.tabs.mcp"),
            SettingsTab::Terminal => knot_core::l10n::t("settings.tabs.appearance"),
            SettingsTab::Keyboard => knot_core::l10n::t("settings.tabs.keyboard"),
        }
    }
}
