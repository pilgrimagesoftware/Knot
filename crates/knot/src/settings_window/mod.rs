use super::*;
/// Opens the settings window, or brings it forward if already open.
pub(crate) fn open_settings_window(handle: &Rc<RefCell<Option<AnyWindowHandle>>>,
                                   settings: knot_core::Settings, cx: &mut App) {
    if let Some(existing) = *handle.borrow()
       && existing.update(cx, |_, window, _| window.activate_window())
                  .is_ok()
    {
        return;
    }
    let options = settings_window_options(cx);
    match cx.open_window(options, move |window, cx| {
                let selected_agent_type = "claude".to_string();
                let initial_options = settings.agent_options
                                              .get(&selected_agent_type)
                                              .cloned()
                                              .unwrap_or_default();
                let agent_options_input = cx.new(|cx| {
                                                InputState::new(window, cx)
                .placeholder("Extra CLI options")
                .default_value(initial_options)
                                            });
                let ai_api_key_input = cx.new(|cx| {
                                             InputState::new(window, cx)
                .placeholder("API key")
                .default_value(settings.ai_api_key.clone())
                                         });
                let autopilot_custom_prompt_input = cx.new(|cx| {
                                                          InputState::new(window, cx)
                .placeholder("Custom prompt")
                .default_value(settings.autopilot_custom_prompt.clone())
                                                      });
                let mcp_port_input = cx.new(|cx| {
                                           InputState::new(window, cx)
                .placeholder("Port")
                .default_value(settings.mcp_server_port.to_string())
                                       });
                let view = cx.new(|cx| {
                                 let agent_options_subscription =
                                     cx.subscribe(&agent_options_input,
                                                  |this: &mut SettingsWindow, _, event, cx| {
                                                      if matches!(event, InputEvent::Change) {
                                                          this.save_agent_options(cx);
                                                      }
                                                  });
                                 let ai_api_key_subscription =
                                     cx.subscribe(&ai_api_key_input,
                                                  |this: &mut SettingsWindow, _, event, cx| {
                                                      if matches!(event, InputEvent::Change) {
                                                          this.save_ai_api_key(cx);
                                                      }
                                                  });
                                 let autopilot_custom_prompt_subscription =
                                     cx.subscribe(&autopilot_custom_prompt_input,
                                                  |this: &mut SettingsWindow, _, event, cx| {
                                                      if matches!(event, InputEvent::Change) {
                                                          this.save_autopilot_custom_prompt(cx);
                                                      }
                                                  });
                                 let mcp_port_subscription =
                                     cx.subscribe(&mcp_port_input,
                                                  |this: &mut SettingsWindow, _, event, cx| {
                                                      if matches!(event, InputEvent::Change) {
                                                          this.save_mcp_port(cx);
                                                      }
                                                  });
                                 SettingsWindow { settings,
                                                  selected_tab: SettingsTab::General,
                                                  selected_agent_type,
                                                  mcp_selected_agent_type: "claude".to_string(),
                                                  agent_options_input,
                                                  ai_api_key_input,
                                                  autopilot_custom_prompt_input,
                                                  mcp_port_input,
                                                  _agent_options_subscription:
                                                      agent_options_subscription,
                                                  _ai_api_key_subscription:
                                                      ai_api_key_subscription,
                                                  _autopilot_custom_prompt_subscription:
                                                      autopilot_custom_prompt_subscription,
                                                  _mcp_port_subscription: mcp_port_subscription }
                             });
                #[cfg(target_os = "macos")]
                {
                    let settings_window = view.clone();
                    cx.spawn(async move |cx| {
                          loop {
                              cx.background_executor()
                                .timer(Duration::from_millis(300))
                                .await;
                              if let Some((target, family, size)) =
                                  native_font_panel::poll_selection()
                              {
                                  cx.update(|app| {
                                        settings_window.update(app, |view, cx| {
                                                           match target {
                                    native_font_panel::Target::Ui => {
                                        view.settings.ui_font_name = family;
                                        view.settings.ui_font_size = size;
                                    }
                                    native_font_panel::Target::Title => {
                                        view.settings.title_font_name = family.clone();
                                        view.settings.title_font_size = size;
                                        let theme = cx.global_mut::<Theme>();
                                        theme.font_family = family.into();
                                        theme.font_size = px(size as f32);
                                    }
                                    native_font_panel::Target::Terminal => {
                                        view.settings.terminal_font_name = family;
                                        view.settings.terminal_font_size = size;
                                    }
                                }
                                                           view.persist();
                                                           cx.notify();
                                                       });
                                    });
                              }
                          }
                      })
                      .detach();
                }
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            }) {
        Ok(window) => *handle.borrow_mut() = Some(window.into()),
        Err(error) => eprintln!("failed to open settings window: {error}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Coding,
    Personas,
    Autopilot,
    Voice,
    Mcp,
    Terminal,
}

impl SettingsTab {
    pub(crate) const ALL: [SettingsTab; 7] = [SettingsTab::General,
                                              SettingsTab::Coding,
                                              SettingsTab::Personas,
                                              SettingsTab::Autopilot,
                                              SettingsTab::Voice,
                                              SettingsTab::Mcp,
                                              SettingsTab::Terminal];

    pub(crate) fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Coding => "Coding",
            SettingsTab::Personas => "Personas",
            SettingsTab::Autopilot => "Autopilot",
            SettingsTab::Voice => "Voice",
            SettingsTab::Mcp => "MCP",
            SettingsTab::Terminal => "Appearance",
        }
    }
}

pub(crate) struct SettingsWindow {
    settings:                              knot_core::Settings,
    selected_tab:                          SettingsTab,
    selected_agent_type:                   String,
    mcp_selected_agent_type:               String,
    agent_options_input:                   Entity<InputState>,
    ai_api_key_input:                      Entity<InputState>,
    autopilot_custom_prompt_input:         Entity<InputState>,
    mcp_port_input:                        Entity<InputState>,
    _agent_options_subscription:           Subscription,
    _ai_api_key_subscription:              Subscription,
    _autopilot_custom_prompt_subscription: Subscription,
    _mcp_port_subscription:                Subscription,
}

impl SettingsWindow {
    fn persist(&self) {
        if let Err(error) = self.settings.persist() {
            eprintln!("failed to persist settings: {error}");
        }
    }

    /// Right-aligned label column width shared by every settings row, so
    /// labels line up across a pane regardless of their length.
    const LABEL_WIDTH: f32 = 200.;

    /// A titled, bordered card grouping related controls. The title is
    /// deliberately larger than row content (`text_lg` vs. the default
    /// `text_base` used by row labels/controls) - a section header should
    /// never read smaller than what it's heading.
    fn group(title: &'static str) -> GroupBox {
        GroupBox::new().outline()
                       .title(div().text_lg().font_semibold().child(title))
    }

    /// A label + control row with the label right-aligned in a fixed-width
    /// column, matching the alignment convention already used by
    /// `AgentEditor`/`PersonaEditor`.
    fn row(label: &'static str, control: impl IntoElement) -> impl IntoElement {
        h_flex().gap_3()
                .items_center()
                .child(div().w(px(Self::LABEL_WIDTH))
                            .flex_shrink_0()
                            .text_right()
                            .child(label))
                .child(control)
    }

    /// Like `row`, but baseline-aligned instead of center-aligned - for rows
    /// whose control is itself text (a read-only value, not a switch/button/
    /// input), so the value's text baseline lines up with the label's.
    fn text_row(label: &'static str, control: impl IntoElement) -> impl IntoElement {
        h_flex().gap_3()
                .items_baseline()
                .child(div().w(px(Self::LABEL_WIDTH))
                            .flex_shrink_0()
                            .text_right()
                            .child(label))
                .child(control)
    }

    /// Muted description text lined up under a row's *control* column,
    /// not spanning the full card width - it explains the control above
    /// it, not the section as a whole.
    fn hint(cx: &Context<Self>, text: &'static str) -> impl IntoElement {
        h_flex().gap_3()
                .child(div().w(px(Self::LABEL_WIDTH)).flex_shrink_0())
                .child(div().flex_1()
                            .min_w_0()
                            .text_sm()
                            .whitespace_normal()
                            .text_color(cx.theme().muted_foreground)
                            .child(text))
    }

    /// Renders `text` in the theme's monospace font, for values that are
    /// literally code/commands/identifiers (install commands, model names).
    fn mono_text(cx: &Context<Self>, text: impl Into<gpui_kit::SharedString>) -> gpui_kit::Div {
        div().text_sm()
             .font_family(cx.theme().mono_font_family.clone())
             .child(text.into())
    }

    /// A small icon-only action button with a tooltip, used for utility
    /// actions (choose/clear/add/edit/delete/copy) instead of a text label -
    /// text buttons read as arbitrary activators, an icon reads as what it
    /// does. `danger` tints destructive actions (clear/delete) red.
    pub(crate) fn icon_button(id: impl Into<gpui_kit::ElementId>, icon_path: &'static str,
                              tooltip: &'static str, danger: bool)
                              -> Button {
        let mut icon = Icon::default().path(icon_path);
        if danger {
            // `.ghost()` and `.danger()` are both button *variants* - only one
            // can apply, and ghost (no background) is what we want here - so
            // tint the icon itself red instead of switching variants.
            icon = icon.text_color(rgb(0xEF4444));
        }
        Button::new(id).icon(icon).tooltip(tooltip).ghost().small()
    }

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

    fn save_agent_options(&mut self, cx: &mut Context<Self>) {
        let value = self.agent_options_input.read(cx).value().to_string();
        self.settings
            .agent_options
            .insert(self.selected_agent_type.clone(), value);
        self.persist();
    }

    pub(crate) fn ai_provider_label(provider: &str) -> &'static str {
        match provider {
            "anthropic" => "Anthropic",
            "google" => "Google",
            _ => "OpenAI",
        }
    }

    /// Hardcoded model for each AI provider (cheapest/fastest options),
    /// matching the Swift reference's `AppSettings.aiModel(for:)`.
    pub(crate) fn ai_model_for(provider: &str) -> &'static str {
        match provider {
            "anthropic" => "claude-haiku-4-5",
            "google" => "gemini-flash-lite-latest",
            "openai" => "gpt-5-mini",
            _ => "",
        }
    }

    pub(crate) fn autopilot_action_label(action: &str) -> &'static str {
        match action {
            "ask" => "Ask me",
            "continue" => "Auto-continue",
            "custom" => "Custom",
            _ => "Mark conversation",
        }
    }

    pub(crate) fn autopilot_action_description(action: &str) -> &'static str {
        match action {
            "ask" => "Show a dialog letting you switch to the agent, dismiss, or auto-continue.",
            "continue" => "Automatically send \"yes, continue\" to the agent.",
            "custom" => {
                "Use your own prompt to decide what to reply. The LLM response is injected \
                 directly into the agent."
            }
            _ => "Set the agent status to indicate input is needed and send a notification.",
        }
    }

    fn select_ai_provider(&mut self, provider: &str, cx: &mut Context<Self>) {
        self.settings.ai_provider = provider.to_string();
        self.persist();
        cx.notify();
    }

    fn select_autopilot_action(&mut self, action: &str, cx: &mut Context<Self>) {
        self.settings.autopilot_action = action.to_string();
        self.persist();
        cx.notify();
    }

    fn save_ai_api_key(&mut self, cx: &mut Context<Self>) {
        self.settings.ai_api_key = self.ai_api_key_input.read(cx).value().to_string();
        self.persist();
    }

    fn save_autopilot_custom_prompt(&mut self, cx: &mut Context<Self>) {
        self.settings.autopilot_custom_prompt = self.autopilot_custom_prompt_input
                                                    .read(cx)
                                                    .value()
                                                    .to_string();
        self.persist();
    }

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

    pub(crate) fn mcp_server_url(port: u16) -> String {
        format!("http://127.0.0.1:{port}")
    }

    /// The command to copy for registering `agent_type` against Knot's MCP
    /// server, ported from the Swift reference's
    /// `MCPCommandView.mcpCommandCopy` (Skwad -> Knot renamed).
    pub(crate) fn mcp_install_command(agent_type: &str, url: &str) -> String {
        match agent_type {
            "claude" => format!("claude mcp add --transport http --scope user knot {url}"),
            "codex" => format!("codex mcp add knot --url {url}"),
            "opencode" => "opencode mcp add".to_string(),
            "gemini" => format!("gemini mcp add --transport http knot {url} --scope user"),
            _ => String::new(),
        }
    }

    fn save_mcp_port(&mut self, cx: &mut Context<Self>) {
        let value = self.mcp_port_input.read(cx).value().to_string();
        if let Ok(port) = value.parse::<u16>() {
            self.settings.mcp_server_port = port;
            self.persist();
        }
    }

    fn select_mcp_agent_type(&mut self, agent_type: &str, cx: &mut Context<Self>) {
        self.mcp_selected_agent_type = agent_type.to_string();
        cx.notify();
    }

    /// Truncates `instructions` to `max_chars`, appending an ellipsis when
    /// truncated so a persona list row stays a single line.
    pub(crate) fn persona_preview(instructions: &str, max_chars: usize) -> String {
        let truncated: String = instructions.chars().take(max_chars).collect();
        if instructions.chars().count() > max_chars {
            format!("{truncated}…")
        }
        else {
            truncated
        }
    }

    fn delete_persona(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if let Err(error) = self.settings.remove_persona(id) {
            eprintln!("failed to remove persona: {error}");
        }
        cx.notify();
    }

    fn restore_default_personas(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.settings.restore_default_personas() {
            eprintln!("failed to restore default personas: {error}");
        }
        cx.notify();
    }
}

impl SettingsWindow {
    /// Target window height for each pane's content, capped so a long
    /// Personas list can't push the window arbitrarily tall - it scrolls
    /// within the cap instead (see `render` below).
    pub(crate) fn pane_target_height(tab: SettingsTab) -> gpui_kit::Pixels {
        match tab {
            SettingsTab::General => px(560.),
            SettingsTab::Coding => px(440.),
            SettingsTab::Personas => px(600.),
            SettingsTab::Autopilot => px(660.),
            SettingsTab::Voice => px(520.),
            SettingsTab::Mcp => px(600.),
            SettingsTab::Terminal => px(380.),
        }
    }

    fn render_tab_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let selected_index = SettingsTab::ALL.iter()
                                             .position(|tab| *tab == self.selected_tab)
                                             .unwrap_or(0);
        TabBar::new("settings-tabs").underline()
                                    .selected_index(selected_index)
                                    .children(SettingsTab::ALL.map(|tab| {
                                                                  Tab::new().label(tab.label())
                                                              }))
                                    .on_click(move |index, window, app| {
                                        let tab = SettingsTab::ALL[*index];
                                        settings_window.update(app, |view, cx| {
                                                           view.selected_tab = tab;
                                                           cx.notify();
                                                       });
                                        window.resize(size(SETTINGS_WINDOW_WIDTH,
                                                           Self::pane_target_height(tab)));
                                    })
    }

    fn render_general(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn render_coding(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn render_personas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let personas: Vec<knot_core::Persona> = self.settings
                                                    .active_personas()
                                                    .into_iter()
                                                    .cloned()
                                                    .collect();

        let list = if personas.is_empty() {
            div().text_sm()
                 .text_color(cx.theme().muted_foreground)
                 .child("No personas defined.")
                 .into_any_element()
        }
        else {
            v_flex()
                    .gap_3()
                    .children(personas.into_iter().enumerate().map(|(index, persona)| {
                        let id = persona.id;
                        let preview = Self::persona_preview(&persona.instructions, 80);
                        h_flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().child(persona.name.clone()))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(preview),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_shrink_0()
                                    .gap_1()
                                    .child(
                                        Self::icon_button(
                                            ("persona-edit", index),
                                            "icons/pencil.svg",
                                            "Edit persona",
                                            false,
                                        )
                                        .on_click({
                                            let parent = settings_window.downgrade();
                                            let persona = persona.clone();
                                            move |_, _, app| {
                                                open_persona_editor(
                                                    parent.clone(),
                                                    Some(persona.clone()),
                                                    app,
                                                );
                                            }
                                        }),
                                    )
                                    .child(
                                        Self::icon_button(
                                            ("persona-delete", index),
                                            "icons/trash.svg",
                                            "Delete persona",
                                            true,
                                        )
                                        .on_click({
                                            let settings_window = settings_window.clone();
                                            let name = persona.name.clone();
                                            move |_, window, app| {
                                                let settings_window = settings_window.clone();
                                                window.open_alert_dialog(app, {
                                                    let name = name.clone();
                                                    move |alert, _, _| {
                                                        let settings_window =
                                                            settings_window.clone();
                                                        alert
                                                        .title("Delete Persona")
                                                        .description(format!(
                                                            "This permanently deletes \"{name}\". \
                                                             This can't be undone."
                                                        ))
                                                        .confirm()
                                                        .on_ok(move |_, _, app| {
                                                            settings_window.update(app, |view, cx| {
                                                                view.delete_persona(id, cx);
                                                            });
                                                            true
                                                        })
                                                    }
                                                });
                                            }
                                        }),
                                    ),
                            )
                    }))
                    .into_any_element()
        };
        // Bounded so the list scrolls in place instead of pushing the group's
        // title/action row (which must stay visible) off the top of the
        // window - same cap philosophy as the window's own per-pane height.
        let list = div().id("personas-list")
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .child(list);

        v_flex().gap_3().child(
            Self::group("Personas")
                .child(
                    h_flex()
                        .justify_between()
                        .child(
                            Self::icon_button(
                                "personas-add",
                                "icons/plus.svg",
                                "Add Persona…",
                                false,
                            )
                            .on_click({
                                let parent = settings_window.downgrade();
                                move |_, _, app| {
                                    open_persona_editor(parent.clone(), None, app);
                                }
                            }),
                        )
                        .child(
                            Self::icon_button(
                                "personas-restore-defaults",
                                "icons/rotate-ccw.svg",
                                "Restore Defaults",
                                false,
                            )
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |_, window, app| {
                                    let settings_window = settings_window.clone();
                                    window.open_alert_dialog(app, move |alert, _, _| {
                                        let settings_window = settings_window.clone();
                                        alert
                                            .title("Restore Defaults")
                                            .description(
                                                "Resets built-in personas to their \
                                                     original name and instructions. \
                                                     Personas you created are not affected.",
                                            )
                                            .confirm()
                                            .on_ok(move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.restore_default_personas(cx);
                                                });
                                                true
                                            })
                                    });
                                }
                            }),
                        ),
                )
                .child(list),
        )
    }

    fn render_autopilot(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let autopilot_enabled = self.settings.autopilot_enabled;
        let ai_provider = self.settings.ai_provider.clone();
        let autopilot_action = self.settings.autopilot_action.clone();
        let provider_label = Self::ai_provider_label(&ai_provider);
        let model_name = Self::ai_model_for(&ai_provider);
        let action_label = Self::autopilot_action_label(&autopilot_action);
        let is_custom_action = autopilot_action == "custom";

        v_flex()
            .gap_3()
            .child(
                Self::group("Enable")
                    .child(Self::row(
                        "Enable autopilot",
                        Switch::new("autopilot-enabled")
                            .checked(autopilot_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.autopilot_enabled = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        "Automatically detect when agents need input and take action — no need \
                         to babysit your agents. Only available with Claude Code.",
                    )),
            )
            .child(
                Self::group("AI Provider")
                    .child(Self::row(
                        "Provider",
                        Button::new("autopilot-provider-picker")
                            .label(provider_label)
                            .dropdown_caret(true)
                            .dropdown_menu({
                                let settings_window = settings_window.clone();
                                move |menu, _, _| {
                                    let mut menu = menu;
                                    for (label, value) in [
                                        ("OpenAI", "openai"),
                                        ("Anthropic", "anthropic"),
                                        ("Google", "google"),
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.select_ai_provider(value, cx);
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .child(Self::row(
                        "API Key",
                        Input::new(&self.ai_api_key_input)
                            .font_family(cx.theme().mono_font_family.clone())
                            .flex_1(),
                    ))
                    .child(Self::text_row(
                        "Model",
                        Self::mono_text(cx, model_name).text_color(cx.theme().muted_foreground),
                    )),
            )
            .child(
                Self::group("Action")
                    .child(Self::row(
                        "When input is detected",
                        Button::new("autopilot-action-picker")
                            .label(action_label)
                            .dropdown_caret(true)
                            .dropdown_menu({
                                let settings_window = settings_window.clone();
                                move |menu, _, _| {
                                    let mut menu = menu;
                                    for (label, value) in [
                                        ("Mark conversation", "mark"),
                                        ("Ask me", "ask"),
                                        ("Auto-continue", "continue"),
                                        ("Custom", "custom"),
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.select_autopilot_action(value, cx);
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .children(is_custom_action.then(|| {
                        Self::row(
                            "Custom prompt",
                            Input::new(&self.autopilot_custom_prompt_input).flex_1(),
                        )
                        .into_any_element()
                    }))
                    .children((!is_custom_action).then(|| {
                        Self::hint(cx, Self::autopilot_action_description(&autopilot_action))
                            .into_any_element()
                    })),
            )
    }

    fn render_voice(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                        "Voice input allows you to speak commands to your agents using \
                     push-to-talk. Hold the configured key to record, release to stop.",
                    ),
            )
            .child(
                Self::group("Engine")
                    .child(Self::row(
                        "Enable voice input",
                        Switch::new("voice-enabled")
                            .checked(voice_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.voice_enabled = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        "Engine",
                        Button::new("voice-engine-picker")
                            .label("Apple SpeechAnalyzer")
                            .disabled(true),
                    ))
                    .child(Self::hint(
                        cx,
                        "Uses on-device speech recognition. No data is sent to the cloud.",
                    )),
            )
            .child(
                Self::group("Input")
                    .child(Self::row(
                        "Push-to-Talk Key",
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .opacity(if voice_enabled { 1.0 } else { 0.5 })
                            .child(key_name),
                    ))
                    .child(Self::row(
                        "Auto-insert transcription",
                        Switch::new("voice-auto-insert")
                            .checked(voice_auto_insert)
                            .disabled(!voice_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.voice_auto_insert = checked;
                                        view.persist();
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

    fn render_mcp(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let mcp_server_enabled = self.settings.mcp_server_enabled;
        let server_url = Self::mcp_server_url(self.settings.mcp_server_port);
        let agent_type_label = Self::agent_type_label(&self.mcp_selected_agent_type);
        let install_command = Self::mcp_install_command(&self.mcp_selected_agent_type, &server_url);

        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        "Knot runs a local MCP server so coding agents can coordinate with each \
                     other and control the app.",
                    ),
            )
            .child(
                Self::group("Server Settings")
                    .child(Self::row(
                        "Enable MCP server",
                        Switch::new("mcp-server-enabled")
                            .checked(mcp_server_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.mcp_server_enabled = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        "Port",
                        Input::new(&self.mcp_port_input).w(px(100.)),
                    ))
                    .child(Self::text_row(
                        "URL",
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Self::mono_text(cx, server_url.clone())
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                Self::icon_button(
                                    "mcp-copy-url",
                                    "icons/copy.svg",
                                    "Copy URL",
                                    false,
                                )
                                .on_click({
                                    let server_url = server_url.clone();
                                    move |_, _, app| {
                                        app.write_to_clipboard(ClipboardItem::new_string(
                                            server_url.clone(),
                                        ));
                                    }
                                }),
                            ),
                    )),
            )
            .child(
                Self::group("Installation Command")
                    .child(Self::row(
                        "Agent",
                        Button::new("mcp-agent-type-picker")
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
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.select_mcp_agent_type(value, cx);
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .child(Self::text_row(
                        "Command",
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .items_start()
                            .child(if install_command.is_empty() {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .child("No manual setup needed.")
                                    .into_any_element()
                            } else {
                                Self::mono_text(cx, install_command.clone())
                                    .flex_1()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .into_any_element()
                            })
                            .child(
                                Self::icon_button(
                                    "mcp-copy-install-command",
                                    "icons/copy.svg",
                                    "Copy command",
                                    false,
                                )
                                .on_click(move |_, _, app| {
                                    if !install_command.is_empty() {
                                        app.write_to_clipboard(ClipboardItem::new_string(
                                            install_command.clone(),
                                        ));
                                    }
                                }),
                            ),
                    )),
            )
    }

    /// A single "Family, Npt" button that opens the OS font panel
    /// (`NSFontPanel`) pre-selected to the current font/size for `target` -
    /// one control picks both, since the panel itself has a size field.
    /// The choice comes back asynchronously via
    /// `native_font_panel::poll_selection`.
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    fn font_picker_button(id: &'static str, target: FontPanelTarget, name: String, size: f64)
                          -> Button {
        Button::new(id).label(format!("{name}, {size:.0}pt"))
                       .on_click(move |_, _, _| {
                           #[cfg(target_os = "macos")]
                           native_font_panel::open(target, &name, size);
                       })
    }

    fn render_appearance(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            Self::group("Fonts")
                .child(Self::row(
                    "UI",
                    Self::font_picker_button(
                        "ui-font-picker",
                        FontPanelTarget::Ui,
                        self.settings.ui_font_name.clone(),
                        self.settings.ui_font_size,
                    ),
                ))
                .child(Self::row(
                    "Title",
                    Self::font_picker_button(
                        "title-font-picker",
                        FontPanelTarget::Title,
                        self.settings.title_font_name.clone(),
                        self.settings.title_font_size,
                    ),
                ))
                .child(Self::row(
                    "Terminal",
                    Self::font_picker_button(
                        "terminal-font-picker",
                        FontPanelTarget::Terminal,
                        self.settings.terminal_font_name.clone(),
                        self.settings.terminal_font_size,
                    ),
                )),
        )
    }
}

pub(crate) fn persona_editor_window_options(title: &'static str, cx: &App) -> WindowOptions {
    WindowOptions { titlebar: Some(gpui_kit::TitlebarOptions { title: Some(title.into()),
                                                               ..Default::default() }),
                    window_bounds: Some(WindowBounds::centered(size(px(460.), px(380.)), cx)),
                    window_min_size: Some(size(px(400.), px(320.))),
                    ..WindowOptions::default() }
}

/// Opens the persona add/edit window. `persona` is `None` for "Add Persona…"
/// and `Some` (fields pre-filled) for a row's edit button.
pub(crate) fn open_persona_editor(parent: WeakEntity<SettingsWindow>,
                                  persona: Option<knot_core::Persona>, cx: &mut App) {
    let editing_id = persona.as_ref().map(|p| p.id);
    let title = if editing_id.is_some() {
        "Edit Persona"
    }
    else {
        "New Persona"
    };
    let name = persona.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    let instructions = persona.as_ref()
                              .map(|p| p.instructions.clone())
                              .unwrap_or_default();
    let options = persona_editor_window_options(title, cx);
    let _ = cx.open_window(options, move |window, cx| {
                  let name_input = cx.new(|cx| {
                                         InputState::new(window, cx).placeholder("Persona name")
                                                                    .default_value(name)
                                     });
                  name_input.update(cx, |state, cx| state.focus(window, cx));
                  let instructions_input = cx.new(|cx| {
                                                 TextareaState::new(window, cx)
                .placeholder("Instructions")
                .default_value(instructions)
                                             });
                  let view = cx.new(|_| PersonaEditor { parent,
                                                        editing_id,
                                                        name_input,
                                                        instructions_input,
                                                        error: None });
                  cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
              });
}

pub(crate) struct PersonaEditor {
    parent:             WeakEntity<SettingsWindow>,
    editing_id:         Option<Uuid>,
    name_input:         Entity<InputState>,
    instructions_input: Entity<TextareaState>,
    error:              Option<String>,
}

impl PersonaEditor {
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some("Enter a persona name.".to_string());
            cx.notify();
            return;
        }
        let instructions = self.instructions_input.read(cx).value().trim().to_string();
        let Some(parent) = self.parent.upgrade()
        else {
            window.remove_window();
            return;
        };
        parent.update(cx, |view, view_cx| {
                  let result = match self.editing_id {
                      Some(id) => view.settings.update_persona(id, name, instructions),
                      None => view.settings.add_persona(name, instructions).map(|_| ()),
                  };
                  if let Err(error) = result {
                      eprintln!("failed to save persona: {error}");
                  }
                  view_cx.notify();
              });
        window.remove_window();
    }
}

impl Render for PersonaEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_3()
            .p_5()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .gap_2()
                    .child(div().w(px(100.)).text_right().child("Name"))
                    .child(Input::new(&self.name_input).flex_1()),
            )
            .child(
                h_flex()
                    .flex_1()
                    .gap_2()
                    .child(div().w(px(100.)).text_right().child("Instructions"))
                    .child(
                        Textarea::new(&self.instructions_input)
                            .flex_1()
                            .h_full()
                            .font_family(cx.theme().mono_font_family.clone()),
                    ),
            )
            .children(
                self.error
                    .as_ref()
                    .map(|error| div().text_sm().child(error.clone())),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-persona-editor")
                            .label("Cancel")
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("save-persona-editor")
                            .label("Save")
                            .primary()
                            .on_click(cx.listener(|editor, _, window, cx| editor.save(window, cx))),
                    ),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.selected_tab {
            SettingsTab::General => self.render_general(cx).into_any_element(),
            SettingsTab::Coding => self.render_coding(cx).into_any_element(),
            SettingsTab::Personas => self.render_personas(cx).into_any_element(),
            SettingsTab::Autopilot => self.render_autopilot(cx).into_any_element(),
            SettingsTab::Voice => self.render_voice(cx).into_any_element(),
            SettingsTab::Mcp => self.render_mcp(cx).into_any_element(),
            SettingsTab::Terminal => self.render_appearance(cx).into_any_element(),
        };

        // Personas manages its own scroll region (only the list scrolls, the
        // title/action row stays pinned) - scrolling the body too would let
        // both containers move at once and make the group's title/border
        // appear to drift.
        let mut settings_body = div().id("settings-body").flex_1();
        settings_body = if matches!(self.selected_tab, SettingsTab::Personas) {
            settings_body.overflow_hidden()
        }
        else {
            settings_body.overflow_y_scroll()
        };

        v_flex().size_full()
                .gap_3()
                .p_4()
                .bg(cx.theme().background)
                .child(self.render_tab_strip(cx))
                .child(settings_body.child(body))
                .children(crate::app_support::root_overlays(window, cx))
    }
}
