use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::PathPromptOptions;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Disableable;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::div;

use crate::settings_window::SettingsWindow;

/// The Coding tab's last adapter version check for one agent type - see
/// [`SettingsWindow::check_for_adapter_update`]/
/// [`SettingsWindow::update_adapter_now`].
#[derive(Debug, Clone, Default)]
pub(crate) enum AdapterUpdateState {
    /// Nothing checked yet since the window opened.
    #[default]
    Idle,
    Checking,
    Checked(knot_terminal::AdapterUpdateStatus),
    Updating,
    /// Holds the check or install failure's message, shown as the row's
    /// hint instead of a version.
    Error(String),
}

impl SettingsWindow {
    /// The name to show for an agent type.
    ///
    /// The roster in `knot_core::agent_type` answers this, so a type added
    /// there is named everywhere at once. An id this build does not know
    /// shows as itself rather than as Claude: the point of #224 is that a
    /// value nothing recognizes must not be indistinguishable from the
    /// default.
    pub(crate) fn agent_type_label(agent_type: &str) -> &str {
        knot_core::agent_type::label(agent_type)
    }

    /// A representative icon per agent type, so a sidebar row is
    /// identifiable at a glance rather than by its one-character avatar.
    ///
    /// Picked to echo each vendor's own mark where the icon set has one
    /// (Claude's asterisk, Gemini's sparkle, GitHub's for Copilot). Keyed
    /// on the same ids as `knot_core::agent_type`'s roster; the fallback
    /// covers the user's own custom commands, which have no mark to echo,
    /// and any type this build does not know.
    ///
    /// Uses the full `gpui_kit::assets` (Lucide) set rather than GPUI
    /// Component's smaller built-in `IconName`, which has no brace,
    /// terminal or sparkle glyph.
    pub(crate) fn agent_type_icon(agent_type: &str) -> gpui_kit::assets::IconName {
        use gpui_kit::assets::IconName;
        match agent_type {
            "claude" => IconName::Asterisk,
            "codex" => IconName::Braces,
            "opencode" => IconName::Terminal,
            "gemini" => IconName::Sparkles,
            "copilot" => IconName::Github,
            "shell" => IconName::SquareTerminal,
            _ => IconName::Bot,
        }
    }

    fn choose_source_folder(&mut self, cx: &mut Context<Self>) {
        let receiver =
            cx.prompt_for_paths(PathPromptOptions { files:       false,
                                                    directories: true,
                                                    multiple:    false,
                                                    prompt:
                                                        Some("Choose Source Folder".into()), });
        let settings_window = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              cx.update(|app| {
                    settings_window.update(app, |view, cx| {
                                       crate::settings_global::write(cx, |settings| {
                                           settings.source_base_folder =
                                               path.to_string_lossy().into_owned();
                                       });
                                       view.persist(cx);
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    fn clear_source_folder(&mut self, cx: &mut Context<Self>) {
        crate::settings_global::write(cx, |settings| settings.source_base_folder.clear());
        self.persist(cx);
        cx.notify();
    }

    fn select_agent_type(&mut self, agent_type: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_agent_type = agent_type.to_string();
        let value = crate::settings_global::read(cx).agent_options
                                                    .get(agent_type)
                                                    .cloned()
                                                    .unwrap_or_default();
        cx.update_entity(&self.agent_options_input, |input, input_cx| {
              input.set_value(value, window, input_cx);
          });
        cx.notify();
    }

    /// Runs [`knot_terminal::check_adapter_update`] for `agent_type`'s
    /// npm-installed adapter and records the result, so the Coding tab can
    /// show "update available" without ever running it unasked (issue
    /// #598's model-list mismatch traces back to the adapter npm package
    /// going stale with nothing to notice).
    fn check_for_adapter_update(&mut self, agent_type: String, cx: &mut Context<Self>) {
        let Some(install) =
            knot_agent_launch::acp_adapter(&agent_type).and_then(|config| config.install)
        else {
            return;
        };
        self.adapter_update
            .insert(agent_type.clone(), AdapterUpdateState::Checking);
        cx.notify();
        let settings_window = cx.entity();
        cx.spawn(async move |_this, cx| {
              let result = knot_terminal::check_adapter_update(install).await;
              cx.update(|app| {
                    settings_window.update(app, |view, cx| {
                                       view.adapter_update
                                           .insert(agent_type, match result {
                                               Ok(status) => AdapterUpdateState::Checked(status),
                                               Err(error) => {
                                                   AdapterUpdateState::Error(error.to_string())
                                               }
                                           });
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    /// Re-installs `agent_type`'s adapter (the same command the auto-install
    /// path runs on a missing binary) and re-checks its version once done,
    /// so the row reflects the new installed version without a second
    /// click.
    fn update_adapter_now(&mut self, agent_type: String, cx: &mut Context<Self>) {
        let Some(install) =
            knot_agent_launch::acp_adapter(&agent_type).and_then(|config| config.install)
        else {
            return;
        };
        self.adapter_update
            .insert(agent_type.clone(), AdapterUpdateState::Updating);
        cx.notify();
        let settings_window = cx.entity();
        cx.spawn(async move |_this, cx| {
              let install_result = knot_terminal::update_adapter(install).await;
              let status = match install_result {
                  Ok(()) => knot_terminal::check_adapter_update(install).await,
                  Err(error) => Err(error),
              };
              cx.update(|app| {
                    settings_window.update(app, |view, cx| {
                                       view.adapter_update
                                           .insert(agent_type, match status {
                                               Ok(status) => AdapterUpdateState::Checked(status),
                                               Err(error) => {
                                                   AdapterUpdateState::Error(error.to_string())
                                               }
                                           });
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    pub(crate) fn save_agent_options(&mut self, cx: &mut Context<Self>) {
        let value = self.agent_options_input.read(cx).value().to_string();
        let agent_type = self.selected_agent_type.clone();
        crate::settings_global::write(cx, |settings| {
            settings.agent_options
                    .insert(agent_type.clone(), value.clone());
        });
        self.persist(cx);
    }

    pub(crate) fn render_coding(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let source_base_folder = crate::settings_global::read(cx).source_base_folder.clone();
        let folder_label = if source_base_folder.is_empty() {
            "Not configured".to_string()
        }
        else {
            source_base_folder
        };
        let agent_type_label = Self::agent_type_label(&self.selected_agent_type);

        v_flex()
            .gap_3()
            .child(
                crate::controls::group(knot_core::l10n::t("settings.coding.source_folder")).child(
                    Self::row(
                        knot_core::l10n::t("settings.coding.folder"),
                        h_flex()
                            .flex_1()
                            .justify_between()
                            .child(div().text_sm().child(folder_label))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        crate::controls::icon_button(
                                            "coding-choose-source-folder",
                                            "icons/folder-open.svg",
                                            "Choose source folder",
                                            false,
                                        )
                                        .on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.choose_source_folder(cx);
                                                })
                                            }
                                        }),
                                    )
                                    .child(
                                        crate::controls::icon_button(
                                            "coding-clear-source-folder",
                                            "icons/x.svg",
                                            "Clear source folder",
                                            true,
                                        )
                                        .on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, window, app| {
                                                let settings_window = settings_window.clone();
                                                window.open_alert_dialog(
                                                    app,
                                                    move |alert, _, _| {
                                                        let settings_window =
                                                            settings_window.clone();
                                                        alert
                                                    .title(knot_core::l10n::t(
                                                        "settings.coding.clear_source_folder",
                                                    ))
                                                    .description(knot_core::l10n::t(
                                                        "settings.coding.clear_source_folder_body",
                                                    ))
                                                    .confirm()
                                                    .on_ok(move |_, _, app| {
                                                        settings_window.update(app, |view, cx| {
                                                            view.clear_source_folder(cx);
                                                        });
                                                        true
                                                    })
                                                    },
                                                );
                                            }
                                        }),
                                    ),
                            ),
                    ),
                ),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.coding.agent_options"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.coding.coding_agent"),
                        // Every type but the user's own custom commands, which
                        // are configured below rather than chosen here - the
                        // Swift reference's `availableAgents` list
                        // (`CodingSettingsView.swift`), read off the roster.
                        Self::dropdown(
                            "coding-agent-type-picker",
                            agent_type_label,
                            knot_core::agent_type::ALL
                                .iter()
                                .filter(|kind| !kind.is_custom)
                                .map(|kind| (kind.label.into(), kind.id))
                                .collect(),
                            settings_window.clone(),
                            |view, value, window, cx| {
                                view.select_agent_type(value, window, cx);
                            },
                        ),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.coding.options"),
                        Input::new(&self.agent_options_input).flex_1(),
                    )),
            )
            .children(self.render_adapter_update(cx))
    }

    /// The Coding tab's "Adapter" group: the installed ACP adapter command
    /// for the selected agent type and its npm version status, or `None`
    /// for a type with no declared npm install (native-ACP agents, and the
    /// user's own custom commands have nothing to check).
    fn render_adapter_update(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let install =
            knot_agent_launch::acp_adapter(&self.selected_agent_type).and_then(|config| {
                                                                         config.install
                                                                     })?;
        let agent_type = self.selected_agent_type.clone();
        let settings_window = cx.entity();
        let state = self.adapter_update
                        .get(&agent_type)
                        .cloned()
                        .unwrap_or_default();

        let (status_text, status_muted, can_update) = match &state {
            AdapterUpdateState::Idle => ("Not checked yet".to_string(), true, false),
            AdapterUpdateState::Checking => ("Checking…".to_string(), true, false),
            AdapterUpdateState::Updating => ("Installing update…".to_string(), true, false),
            AdapterUpdateState::Error(message) => (message.clone(), false, false),
            AdapterUpdateState::Checked(status) => {
                let installed = status.installed_version
                                      .as_deref()
                                      .unwrap_or("not installed");
                if status.update_available {
                    (format!("{installed} -> {} available", status.latest_version), false, true)
                }
                else {
                    (format!("{installed} (up to date)"), true, false)
                }
            }
        };
        let busy = matches!(state,
                            AdapterUpdateState::Checking | AdapterUpdateState::Updating);

        Some(
            crate::controls::group("Adapter")
                .child(Self::text_row(
                    "Command",
                    Self::mono_text(cx, install.command).text_color(cx.theme().muted_foreground),
                ))
                .child(Self::text_row(
                    "Version",
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child({
                            let mut text = div().text_sm().child(status_text);
                            if status_muted {
                                text = text.text_color(cx.theme().muted_foreground);
                            } else {
                                text = text.text_color(cx.theme().danger);
                            }
                            text
                        })
                        .child(
                            crate::controls::icon_button(
                                "coding-check-adapter-update",
                                "icons/refresh-cw.svg",
                                "Check for updates",
                                false,
                            )
                            .disabled(busy)
                            .on_click({
                                let settings_window = settings_window.clone();
                                let agent_type = agent_type.clone();
                                move |_, _, app| {
                                    settings_window.update(app, |view, cx| {
                                        view.check_for_adapter_update(agent_type.clone(), cx);
                                    });
                                }
                            }),
                        )
                        .children(can_update.then(|| {
                            Button::new("coding-update-adapter")
                                .label("Update")
                                .small()
                                .disabled(busy)
                                .on_click({
                                    let settings_window = settings_window.clone();
                                    let agent_type = agent_type.clone();
                                    move |_, _, app| {
                                        settings_window.update(app, |view, cx| {
                                            view.update_adapter_now(agent_type.clone(), cx);
                                        });
                                    }
                                })
                        })),
                )),
        )
    }
}
