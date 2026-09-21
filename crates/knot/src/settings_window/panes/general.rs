use super::super::*;

impl SettingsWindow {
    pub(crate) fn appearance_label(mode: &str) -> &'static str {
        match mode {
            "system" => "System",
            "light" => "Light",
            "dark" => "Dark",
            _ => "Auto",
        }
    }

    /// "Restore last conversation" only takes effect when layout restore is
    /// on; gate its toggle on that rather than hiding it.
    pub(crate) fn restore_conversation_toggle_enabled(restore_layout_on_launch: bool) -> bool {
        restore_layout_on_launch
    }

    pub(crate) fn render_general(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let restore_layout_on_launch = self.settings.restore_layout_on_launch;
        let restore_conversation_on_launch = self.settings.restore_conversation_on_launch;
        let keep_in_menu_bar = self.settings.keep_in_menu_bar;
        let desktop_notifications_enabled = self.settings.desktop_notifications_enabled;
        let agent_panel_shift_enter_sends = self.settings.agent_panel_shift_enter_sends;
        let appearance_label = Self::appearance_label(&self.settings.appearance_mode);

        v_flex()
            .gap_3()
            .child(
                Self::group("Appearance")
                    .child(Self::row(
                        "Appearance",
                        Button::new("appearance-picker")
                            .label(appearance_label)
                            .dropdown_caret(true)
                            .dropdown_menu({
                                let settings_window = settings_window.clone();
                                move |menu, _, _| {
                                    let mut menu = menu;
                                    for (label, value) in [
                                        ("Auto", "auto"),
                                        ("System", "system"),
                                        ("Light", "light"),
                                        ("Dark", "dark"),
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, _| {
                                                    view.settings.appearance_mode =
                                                        value.to_string();
                                                    view.persist();
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        "Derives color scheme from terminal background color.",
                    )),
            )
            .child(
                Self::group("Startup")
                    .child(Self::row(
                        "Restore agents on launch",
                        Switch::new("restore-layout-on-launch")
                            .checked(restore_layout_on_launch)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.restore_layout_on_launch = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        "Restore last conversation",
                        Switch::new("restore-conversation-on-launch")
                            .checked(restore_conversation_on_launch)
                            .disabled(!Self::restore_conversation_toggle_enabled(
                                restore_layout_on_launch,
                            ))
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.restore_conversation_on_launch = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        "Keep running in menu bar when closed",
                        Switch::new("keep-in-menu-bar")
                            .checked(keep_in_menu_bar)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.keep_in_menu_bar = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    )),
            )
            .child(
                Self::group("Notifications").child(Self::row(
                    "Desktop notifications",
                    Switch::new("desktop-notifications-enabled")
                        .checked(desktop_notifications_enabled)
                        .on_click({
                            let settings_window = settings_window.clone();
                            move |checked, _, app| {
                                let checked = *checked;
                                settings_window.update(app, |view, _| {
                                    view.settings.desktop_notifications_enabled = checked;
                                    view.persist();
                                })
                            }
                        }),
                )),
            )
            .child(
                Self::group("Agent Panel")
                    .child(Self::row(
                        "Shift+Enter to send",
                        Switch::new("agent-panel-shift-enter-sends")
                            .checked(agent_panel_shift_enter_sends)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.agent_panel_shift_enter_sends = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        "When off, Enter sends the message and Shift+Enter adds a newline.",
                    )),
            )
    }
}
