use super::super::*;

impl SettingsWindow {
    pub(crate) fn agent_type_label(agent_type: &str) -> &'static str {
        match agent_type {
            "codex" => "Codex",
            "opencode" => "OpenCode",
            "gemini" => "Gemini",
            "copilot" => "Copilot",
            "custom1" => "Custom 1",
            "custom2" => "Custom 2",
            "shell" => "Shell",
            _ => "Claude",
        }
    }

    /// A representative icon per agent type, so a sidebar row is
    /// identifiable at a glance rather than by its one-character avatar.
    /// Picked to echo each vendor's own mark where the icon set has one
    /// (Claude's asterisk, Gemini's sparkle, GitHub's for Copilot);
    /// everything else falls back to a generic bot.
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
                                       view.settings.source_base_folder =
                                           path.to_string_lossy().into_owned();
                                       view.persist();
                                       cx.notify();
                                   });
                });
          })
          .detach();
    }

    fn clear_source_folder(&mut self, cx: &mut Context<Self>) {
        self.settings.source_base_folder.clear();
        self.persist();
        cx.notify();
    }

    fn select_agent_type(&mut self, agent_type: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_agent_type = agent_type.to_string();
        let value = self.settings
                        .agent_options
                        .get(agent_type)
                        .cloned()
                        .unwrap_or_default();
        cx.update_entity(&self.agent_options_input, |input, input_cx| {
              input.set_value(value, window, input_cx);
          });
        cx.notify();
    }

    pub(crate) fn save_agent_options(&mut self, cx: &mut Context<Self>) {
        let value = self.agent_options_input.read(cx).value().to_string();
        self.settings
            .agent_options
            .insert(self.selected_agent_type.clone(), value);
        self.persist();
    }

    pub(crate) fn render_coding(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let source_base_folder = self.settings.source_base_folder.clone();
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
                Self::group("Source Folder").child(Self::row(
                    "Folder",
                    h_flex()
                        .flex_1()
                        .justify_between()
                        .child(div().text_sm().child(folder_label))
                        .child(
                            h_flex()
                                .gap_1()
                                .child(
                                    Self::icon_button(
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
                                    Self::icon_button(
                                        "coding-clear-source-folder",
                                        "icons/x.svg",
                                        "Clear source folder",
                                        true,
                                    )
                                    .on_click({
                                        let settings_window = settings_window.clone();
                                        move |_, window, app| {
                                            let settings_window = settings_window.clone();
                                            window.open_alert_dialog(app, move |alert, _, _| {
                                                let settings_window = settings_window.clone();
                                                alert
                                                    .title("Clear Source Folder")
                                                    .description(
                                                        "The source folder path will be cleared.",
                                                    )
                                                    .confirm()
                                                    .on_ok(move |_, _, app| {
                                                        settings_window.update(app, |view, cx| {
                                                            view.clear_source_folder(cx);
                                                        });
                                                        true
                                                    })
                                            });
                                        }
                                    }),
                                ),
                        ),
                )),
            )
            .child(
                Self::group("Agent Options")
                    .child(Self::row(
                        "Coding agent",
                        Button::new("coding-agent-type-picker")
                            .label(agent_type_label)
                            .dropdown_caret(true)
                            .dropdown_menu({
                                let settings_window = settings_window.clone();
                                move |menu, _, _| {
                                    let mut menu = menu;
                                    for (label, value) in [
                                        ("Claude", "claude"),
                                        ("Codex", "codex"),
                                        ("OpenCode", "opencode"),
                                        ("Gemini", "gemini"),
                                        ("Copilot", "copilot"),
                                        ("Shell", "shell"),
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, window, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.select_agent_type(value, window, cx);
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .child(Self::row(
                        "Options",
                        Input::new(&self.agent_options_input).flex_1(),
                    )),
            )
    }
}
