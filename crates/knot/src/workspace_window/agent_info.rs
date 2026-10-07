//! Which ACP agent and version a Panel-mode agent is running (issue #591):
//! the header's info button and the Agents menu's Agent Info dialog.
//!
//! Both read the `agentInfo` the adapter reported on `initialize`, which the
//! session already holds - nothing here spawns a process or touches disk, so
//! the header can ask on every frame.

use gpui_kit::App;
use gpui_kit::IntoElement;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use uuid::Uuid;

use crate::panel_session::AgentIdentity;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// What the agent's ACP session reported about itself. `NotConnected`
    /// for an agent with no Panel-mode session, as well as one still
    /// connecting.
    pub(in crate::workspace_window) fn agent_identity(&self, id: Uuid) -> AgentIdentity {
        self.panel_sessions
            .get(&id)
            .map_or(AgentIdentity::NotConnected, |slot| {
                slot.lock().agent_identity()
            })
    }

    /// The header's info button: its tooltip names the agent and version,
    /// and clicking it opens the same dialog as the Agents menu's Agent Info.
    ///
    /// Drawn only once the session is connected. Before that there is
    /// nothing to report, and a Terminal-mode agent never has anything.
    pub(in crate::workspace_window) fn agent_info_button(&self, id: Uuid)
                                                         -> Option<gpui_kit::AnyElement> {
        let identity = self.agent_identity(id);
        if identity == AgentIdentity::NotConnected {
            return None;
        }
        let summary = identity.summary();
        Some(Button::new(("agent-info", id.as_u128() as u64)).icon(IconName::Info)
                                                             .ghost()
                                                             .small()
                                                             .tooltip(summary)
                                                             .on_click(move |_, window, app| {
                                                                 show_agent_info(window, app,
                                                                                 &identity);
                                                             })
                                                             .into_any_element())
    }
}

/// Opens the Agent Info dialog: one informational line and an OK button.
///
/// Deferred because the Agents menu is one of its two callers. macOS
/// dispatches a menu action inside the active window's update, so opening a
/// dialog on that same window inline is a re-entrant update gpui refuses
/// (`knot-ui-conventions`, "Dialogs from menu actions").
pub(in crate::workspace_window) fn show_agent_info(window: &mut Window, app: &mut App,
                                                   identity: &AgentIdentity) {
    let description = identity.summary();
    window.defer(app, move |window, app| {
              window.open_alert_dialog(app, move |alert, _, _| {
                        alert.title(knot_core::l10n::t("panel.agent_info.title"))
                             .description(description.clone())
                    });
          });
}
