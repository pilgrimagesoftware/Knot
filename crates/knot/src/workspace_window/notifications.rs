//! Desktop notifications: agents that need the user's attention, and the
//! MCP server that supervision could not keep up.
//!
//! Contract: `openspec/specs/desktop-notifications/spec.md`, and the
//! `desktop-notifications` delta of
//! `openspec/changes/supervise-mcp-server/` for the server's.
//!
//! The MCP tool catalog pushes `(agent id, message)` onto the shared
//! awaiting-input queue whenever an agent's hook or ACP session reports
//! Awaiting input. Each workspace window drains the entries for *its own*
//! agents, so exactly one window handles each - and it is the window that
//! knows whether the agent is the one already on screen.
//!
//! NOTE: `UNUserNotificationCenter` aborts outside an app bundle, so gpui
//! defers both the authorization prompt and delivery to the first posted
//! notification. Nothing appears under `cargo run`; verifying this path
//! needs `make package` and `Knot.app`.

use gpui_kit::{Context, SystemNotification};
use uuid::Uuid;

use crate::app_state::{notification_body, should_notify, should_show_awaiting_notice};
use crate::app_support::AwaitingInput;
use crate::mcp_status::{MCP_FAILURE_NOTIFICATION_TAG, McpServerStatus, should_notify_mcp_failure};
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Announces that supervision could not keep the MCP server up.
    ///
    /// Unlike every other notification here this one names no agent: it is
    /// about the server every agent reaches Knot through, so a dead server
    /// is noticed without opening settings.
    ///
    /// The marker is taken atomically, which is what makes this safe to run
    /// from every open window: the first window to poll gets it and the
    /// rest find the slot empty, so one episode raises exactly one
    /// notification however many windows are open. With no window open the
    /// marker waits in the slot.
    pub(in crate::workspace_window) fn raise_mcp_failure_notification(&mut self,
                                                                      cx: &mut Context<Self>) {
        if !cx.has_global::<McpServerStatus>() {
            return;
        }
        let claimed = cx.global::<McpServerStatus>().claim_failure();
        if !should_notify_mcp_failure(crate::settings_global::read(cx).desktop_notifications_enabled, claimed) {
            return;
        }
        cx.show_system_notification(SystemNotification {
              tag:     MCP_FAILURE_NOTIFICATION_TAG.into(),
              title:   knot_core::l10n::t("mcp_server.failure_title").into(),
              body:    knot_core::l10n::t("mcp_server.failure_body").into(),
              actions: Vec::new(),
          });
    }

    /// Drains the awaiting-input queue and raises a notification for each
    /// entry this workspace owns.
    ///
    /// Entries for other workspaces are put back, so whichever window owns
    /// them can take them on its own poll. An agent whose window has closed
    /// has had its session torn down with it and cannot produce new entries,
    /// so nothing accumulates unclaimed.
    pub(in crate::workspace_window) fn raise_awaiting_notifications(&mut self,
                                                                    cx: &mut Context<Self>) {
        if !cx.has_global::<AwaitingInput>() {
            return;
        }
        let mine = self.workspace_agent_ids();
        let claimed = {
            let queue = cx.global::<AwaitingInput>().0.clone();
            let mut queue = queue.lock();
            let (claimed, rest): (Vec<_>, Vec<_>) =
                std::mem::take(&mut *queue).into_iter()
                                           .partition(|(id, _)| mine.contains(id));
            *queue = rest;
            claimed
        };

        for (id, message) in claimed {
            let message = message.unwrap_or_default();
            // The selected agent is only "visible" while this window is the
            // one the user is looking at; a background window's selection is
            // not on screen, and the spec's suppression is about what the
            // user can already see.
            let visible_agent = cx.active_window()
                                  .is_some_and(|active| active == self.window_handle)
                                  .then_some(self.selected_agent)
                                  .flatten();
            if !should_show_awaiting_notice(visible_agent,
                                            id,
                                            &message,
                                            self.notified_awaiting.get(&id))
            {
                continue;
            }
            self.notified_awaiting.insert(id, message.clone());
            if !should_notify(crate::settings_global::read(cx).desktop_notifications_enabled,
                              true)
            {
                continue;
            }
            let Some(name) = self.store.lock().agent(id).map(|agent| agent.name.clone())
            else {
                continue;
            };
            cx.show_system_notification(SystemNotification {
                  // The agent id, so a click can route back to it - see
                  // `app_state::notification_response_agent_id`. Reusing it as
                  // the tag also replaces a still-showing notification for the
                  // same agent rather than stacking a second one.
                  tag:     id.to_string().into(),
                  title:   format!("Knot - {name}").into(),
                  body:    notification_body(&message).to_string().into(),
                  actions: Vec::new(),
              });
        }
    }

    /// Forgets what `id` was last notified about, so a restarted agent is
    /// announced again rather than being taken for a repeat.
    pub(in crate::workspace_window) fn forget_awaiting_notification(&mut self, id: Uuid) {
        self.notified_awaiting.remove(&id);
    }
}
