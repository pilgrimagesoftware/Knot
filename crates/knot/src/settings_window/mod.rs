use super::*;

mod controls;
/// Public within the crate because its rule is asserted directly by
/// `tests::settings_font_preview`; the panes themselves are not.
pub(crate) mod font;
mod panes;
mod persona_editor;

/// Opens the settings window, or brings it forward if already open.
pub(crate) fn open_settings_window(handle: &Rc<RefCell<Option<AnyWindowHandle>>>,
                                   settings: knot_core::Settings,
                                   store: Arc<Mutex<knot_agents::AgentStore>>, cx: &mut App) {
    if let Some(existing) = *handle.borrow()
       && existing.update(cx, |_, window, _| window.activate_window())
                  .is_ok()
    {
        return;
    }
    let options = settings_window_options(cx);
    match cx.open_window(options, move |window, cx| {
                // Every window tracks the OS appearance, so a light/dark flip
                // re-resolves the system palette and repaints.
                observe_system_appearance(window);
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
                                                  store,
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
                                .timer(consts::FONT_PANEL_POLL_INTERVAL)
                                .await;
                              if let Some((target, family, size)) =
                                  native_font_panel::poll_selection()
                              {
                                  cx.update(|app| {
                                        settings_window.update(app, |view, cx| {
                                                           match target {
                                    native_font_panel::Target::Ui => {
                                        view.settings.ui_font_name = family.clone();
                                        view.settings.ui_font_size = size;
                                        // The UI font is the app-wide
                                        // default, so the live theme carries
                                        // it: every open window redraws in
                                        // the new family without reopening.
                                        let theme = cx.global_mut::<Theme>();
                                        theme.font_family = family.into();
                                        theme.font_size = px(size as f32);
                                    }
                                    native_font_panel::Target::Title => {
                                        // Read per frame at the sites that
                                        // draw titles and headers, so nothing
                                        // global needs updating here.
                                        view.settings.title_font_name = family;
                                        view.settings.title_font_size = size;
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
                cx.new(|cx| Root::new(view, window, cx))
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
    settings: knot_core::Settings,
    /// The live agent store, for questions this window's own `Settings`
    /// snapshot can't answer truthfully - whether a persona is still
    /// assigned to an agent, which changes while this window is open.
    store: Arc<Mutex<knot_agents::AgentStore>>,
    selected_tab: SettingsTab,
    selected_agent_type: String,
    mcp_selected_agent_type: String,
    agent_options_input: Entity<InputState>,
    ai_api_key_input: Entity<InputState>,
    autopilot_custom_prompt_input: Entity<InputState>,
    mcp_port_input: Entity<InputState>,
    _agent_options_subscription: Subscription,
    _ai_api_key_subscription: Subscription,
    _autopilot_custom_prompt_subscription: Subscription,
    _mcp_port_subscription: Subscription,
}

impl SettingsWindow {
    fn persist(&self) {
        if let Err(error) = self.settings.persist() {
            eprintln!("failed to persist settings: {error}");
        }
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
