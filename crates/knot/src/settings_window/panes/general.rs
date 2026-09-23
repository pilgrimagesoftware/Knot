use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::Disableable;
use gpui_kit::base::v_flex;
use gpui_kit::component::switch::Switch;
use knot_core::AppearanceMode;

use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    /// Exhaustive: adding a variant to [`AppearanceMode`] fails to compile
    /// here rather than silently rendering as "Auto".
    pub(crate) fn appearance_label(mode: AppearanceMode) -> &'static str {
        match mode {
            AppearanceMode::Auto => "Auto",
            AppearanceMode::System => "System",
            AppearanceMode::Light => "Light",
            AppearanceMode::Dark => "Dark",
        }
    }

    /// "Restore last conversation" only takes effect when layout restore is
    /// on; gate its toggle on that rather than hiding it.
    pub(crate) fn restore_conversation_toggle_enabled(restore_layout_on_launch: bool) -> bool {
        restore_layout_on_launch
    }

    /// How the window looks.
    fn appearance_group(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings_window = cx.entity();
        let appearance_label =
            Self::appearance_label(crate::settings_global::read(cx).appearance_mode);
        crate::controls::group(knot_core::l10n::t("settings.general.appearance"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.general.appearance"),
                        // Driven off `AppearanceMode::ALL`, so a new variant
                        // appears in the picker without anyone remembering to
                        // add it.
                        Self::dropdown("appearance-picker",
                                       appearance_label,
                                       AppearanceMode::ALL.iter()
                                                          .copied()
                                                          .map(|mode| {
                                                              (Self::appearance_label(mode).into(),
                                                               mode)
                                                          })
                                                          .collect(),
                                       settings_window.clone(),
                                       |view, mode, _, cx| {
                                           crate::settings_global::write(cx, |settings| {
                                               settings.appearance_mode = *mode;
                                           });
                                           // Writes, then hands the new
                                           // preferences to open workspace
                                           // windows (#238).
                                           view.persist(cx);
                                           // Still needed alongside it: the
                                           // broadcast refreshes workspace
                                           // windows' settings snapshots,
                                           // and the theme is an app-wide
                                           // global that no snapshot owns.
                                           // Persisting alone is what made
                                           // this picker inert.
                                           crate::appearance::set(*mode, cx);
                                       }),
                    ))
                    .child(Self::hint(
                        cx,
                        knot_core::l10n::t("settings.general.appearance_hint"),
                    ))
    }

    /// What the app restores when it launches.
    fn startup_group(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings_window = cx.entity();
        let restore_layout_on_launch = crate::settings_global::read(cx).restore_layout_on_launch;
        let restore_conversation_on_launch =
            crate::settings_global::read(cx).restore_conversation_on_launch;
        let keep_in_menu_bar = crate::settings_global::read(cx).keep_in_menu_bar;
        crate::controls::group(knot_core::l10n::t("settings.general.startup"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.general.restore_agents"),
                        Switch::new("restore-layout-on-launch")
                            .checked(restore_layout_on_launch)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        crate::settings_global::write(cx, |settings| {
                                            settings.restore_layout_on_launch = checked;
                                        });
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.general.restore_conversation"),
                        Switch::new("restore-conversation-on-launch")
                            .checked(restore_conversation_on_launch)
                            .disabled(!Self::restore_conversation_toggle_enabled(
                                restore_layout_on_launch,
                            ))
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        crate::settings_global::write(cx, |settings| {
                                            settings.restore_conversation_on_launch = checked;
                                        });
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.general.keep_in_menu_bar"),
                        Switch::new("keep-in-menu-bar")
                            .checked(keep_in_menu_bar)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        crate::settings_global::write(cx, |settings| {
                                            settings.keep_in_menu_bar = checked;
                                        });
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
    }

    /// Whether an agent waiting for input raises a desktop notification.
    fn notifications_group(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings_window = cx.entity();
        let desktop_notifications_enabled =
            crate::settings_global::read(cx).desktop_notifications_enabled;
        crate::controls::group(knot_core::l10n::t("settings.general.notifications")).child(Self::row(
                    knot_core::l10n::t("settings.general.desktop_notifications"),
                    Switch::new("desktop-notifications-enabled")
                        .checked(desktop_notifications_enabled)
                        .on_click({
                            let settings_window = settings_window.clone();
                            move |checked, _, app| {
                                let checked = *checked;
                                settings_window.update(app, |view, cx| {
                                    crate::settings_global::write(cx, |settings| {
                                        settings.desktop_notifications_enabled = checked;
                                    });
                                    view.persist(cx);
                                })
                            }
                        }),
                ))
    }

    /// How the agent panel's composer and tool calls behave.
    fn agent_panel_group(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let settings_window = cx.entity();
        let agent_panel_shift_enter_sends =
            crate::settings_global::read(cx).agent_panel_shift_enter_sends;
        let agent_panel_compact_tool_calls =
            crate::settings_global::read(cx).agent_panel_compact_tool_calls;
        // Bound here rather than inline: `row`/`hint` borrow their text,
        // so a `t(..)` temporary in the call would not outlive it.
        let compact_tool_calls_label = knot_core::l10n::t("settings.compact_tool_calls");
        let compact_tool_calls_hint = knot_core::l10n::t("settings.compact_tool_calls_hint");
        crate::controls::group(knot_core::l10n::t("settings.general.agent_panel"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.general.shift_enter_to_send"),
                        Switch::new("agent-panel-shift-enter-sends")
                            .checked(agent_panel_shift_enter_sends)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        crate::settings_global::write(cx, |settings| {
                                            settings.agent_panel_shift_enter_sends = checked;
                                        });
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        knot_core::l10n::t("settings.general.shift_enter_hint"),
                    ))
                    // New copy goes through the catalogue, per
                    // `.claude/rules/rust-structure.md` - this pane's older
                    // literals are the pattern not to follow.
                    .child(Self::row(
                        compact_tool_calls_label,
                        Switch::new("agent-panel-compact-tool-calls")
                            .checked(agent_panel_compact_tool_calls)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, cx| {
                                        crate::settings_global::write(cx, |settings| {
                                            settings.agent_panel_compact_tool_calls = checked;
                                        });
                                        view.persist(cx);
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        compact_tool_calls_hint,
                    ))
    }

    pub(crate) fn render_general(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3()
                .child(self.appearance_group(cx))
                .child(self.startup_group(cx))
                .child(self.notifications_group(cx))
                .child(self.agent_panel_group(cx))
    }
}
