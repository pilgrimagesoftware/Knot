//! The settings window entity and how it opens: the inputs it owns, the
//! subscriptions that save them as they change, and the macOS font-panel
//! poll that feeds font choices back in.
//!
//! What it draws lives in `super::render` and the `panes` modules.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Entity;
use gpui_kit::Subscription;
use gpui_kit::component::Root;
// macOS-only: used solely by the native font-panel poll below.
#[cfg(target_os = "macos")]
use gpui_kit::component::Theme;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
#[cfg(target_os = "macos")]
use gpui_kit::px;
use parking_lot::Mutex;

use super::tab::SettingsTab;
// macOS-only: the module it names is `cfg(target_os = "macos")`, and so
// is every use of it here.
#[cfg(target_os = "macos")]
use crate::app_support::native_font_panel;
use crate::app_support::observe_system_appearance;
use crate::consts;
use crate::mcp_status::McpServerStatus;
use crate::window_options::settings_window_options;

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
                let selected_agent_type = knot_core::agent_type::DEFAULT.to_string();
                let initial_options = settings.agent_options
                                              .get(&selected_agent_type)
                                              .cloned()
                                              .unwrap_or_default();
                let agent_options_input = cx.new(|cx| {
                                                InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("settings.input.extra_cli_options"))
                .default_value(initial_options)
                                            });
                let ai_api_key_input = cx.new(|cx| {
                                             InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("settings.input.api_key"))
                .default_value(settings.ai_api_key.clone())
                                         });
                let autopilot_custom_prompt_input = cx.new(|cx| {
                                                          InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("settings.input.custom_prompt"))
                .default_value(settings.autopilot_custom_prompt.clone())
                                                      });
                let mcp_port_input = cx.new(|cx| {
                                           InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("settings.input.port"))
                .default_value(settings.mcp_server_port.to_string())
                                       });
                // The state to open on, so the row is right on the first
                // frame rather than one poll tick later.
                let mcp_state = current_mcp_state(cx);
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
                                                  mcp_selected_agent_type:
                                                      knot_core::agent_type::DEFAULT.to_string(),
                                                  agent_options_input,
                                                  ai_api_key_input,
                                                  autopilot_custom_prompt_input,
                                                  mcp_port_input,
                                                  last_mcp_state: mcp_state,
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
                                                           view.persist(cx);
                                                           cx.notify();
                                                       });
                                    });
                              }
                          }
                      })
                      .detach();
                }
                spawn_mcp_state_poll(view.clone(), cx);
                cx.new(|cx| Root::new(view, window, cx))
            }) {
        Ok(window) => *handle.borrow_mut() = Some(window.into()),
        Err(error) => eprintln!("failed to open settings window: {error}"),
    }
}

/// The MCP server's state as the application last mirrored it, or
/// [`knot_mcp::ServerState::Disabled`] when nothing has reported one -
/// which is the state of a server configuration has turned off.
fn current_mcp_state(cx: &App) -> knot_mcp::ServerState {
    if cx.has_global::<McpServerStatus>() {
        cx.global::<McpServerStatus>().state()
    }
    else {
        knot_mcp::ServerState::Disabled
    }
}

/// Starts the MCP tab's state poll, which runs for the window's lifetime.
///
/// The supervisor publishes its state over a channel on its own thread and
/// GPUI cannot await one, so the row follows a change by re-reading the
/// mirror. It asks for a repaint only when the state differs from the one
/// last drawn: a window left open on another tab would otherwise repaint
/// twice a second for nothing.
fn spawn_mcp_state_poll(view: Entity<SettingsWindow>, cx: &mut App) {
    cx.spawn(async move |cx| {
          loop {
              cx.background_executor()
                .timer(consts::MCP_STATE_POLL_INTERVAL)
                .await;
              cx.update(|app| {
                    let state = current_mcp_state(app);
                    view.update(app, |view, cx| {
                            if SettingsWindow::mcp_state_changed(&mut view.last_mcp_state, state) {
                                cx.notify();
                            }
                        });
                });
          }
      })
      .detach();
}

pub(crate) struct SettingsWindow {
    pub(super) settings: knot_core::Settings,
    /// The live agent store, for questions this window's own `Settings`
    /// snapshot can't answer truthfully - whether a persona is still
    /// assigned to an agent, which changes while this window is open.
    pub(super) store: Arc<Mutex<knot_agents::AgentStore>>,
    pub(super) selected_tab: SettingsTab,
    pub(super) selected_agent_type: String,
    pub(super) mcp_selected_agent_type: String,
    pub(super) agent_options_input: Entity<InputState>,
    pub(super) ai_api_key_input: Entity<InputState>,
    pub(super) autopilot_custom_prompt_input: Entity<InputState>,
    pub(super) mcp_port_input: Entity<InputState>,
    /// The MCP server's state as the MCP tab last drew it. The poll
    /// compares against this rather than repainting every tick.
    pub(super) last_mcp_state: knot_mcp::ServerState,
    pub(super) _agent_options_subscription: Subscription,
    pub(super) _ai_api_key_subscription: Subscription,
    pub(super) _autopilot_custom_prompt_subscription: Subscription,
    pub(super) _mcp_port_subscription: Subscription,
}

impl SettingsWindow {
    /// Write the preferences, then hand them to the windows already drawing
    /// the old values.
    ///
    /// The delivery is here rather than at each of the twenty call sites
    /// because every one of them has the same obligation: a setting that is
    /// stored and not delivered is the shape of issue #238, and it is not
    /// visible at the call site that it was missed.
    pub(super) fn persist(&self, cx: &mut App) {
        if let Err(error) = self.settings.persist_preferences() {
            eprintln!("failed to persist settings: {error}");
            return;
        }
        crate::settings_broadcast::preferences_changed(cx);
    }
}
