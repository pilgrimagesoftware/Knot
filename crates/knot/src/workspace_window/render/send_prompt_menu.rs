//! "Send prompt to": hands an issue or an OpenSpec change to one of the
//! workspace's agents from the Changes view (#504).
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/work-item-prompts/spec.md`.
//! The same submenu sits in both menus of both tabs, so it is built here once
//! from a [`WorkItemRef`] and the workspace's agents.

use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::{App, Context, Entity, Window};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// What a prompt names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) enum WorkItemRef {
    /// An issue, by its URL.
    Issue(String),
    /// An OpenSpec change, by its name.
    Change(String),
}

impl WorkItemRef {
    /// The prompt sent for this item - one localized sentence with the URL
    /// or name substituted.
    pub(in crate::workspace_window) fn prompt_text(&self) -> String {
        match self {
            Self::Issue(url) => knot_core::l10n::t_with("changes_view.prompt.issue", &[("url", url)]),
            Self::Change(name) => {
                knot_core::l10n::t_with("changes_view.prompt.change", &[("name", name)])
            }
        }
    }
}

/// One agent the submenu lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workspace_window) struct SendTarget {
    pub(in crate::workspace_window) id:      Uuid,
    pub(in crate::workspace_window) name:    String,
    /// Whether it can take a prompt now. A stopped agent is listed but
    /// disabled, so the list keeps its shape as agents start and stop.
    pub(in crate::workspace_window) enabled: bool,
}

/// `agents` in alphabetical order by name, ignoring case - the submenu's
/// order. Ties keep their given order.
pub(in crate::workspace_window) fn send_targets(mut agents: Vec<SendTarget>) -> Vec<SendTarget> {
    agents.sort_by_key(|target| target.name.to_lowercase());
    agents
}

impl WorkspaceWindow {
    /// This workspace's agents as the submenu lists them.
    pub(in crate::workspace_window) fn prompt_targets(&self) -> Vec<SendTarget> {
        // The ids first: `workspace_agent_ids` takes the store lock itself,
        // and `parking_lot`'s mutex is not re-entrant, so asking for them
        // while holding it deadlocks the window.
        let ids = self.workspace_agent_ids();
        let agents = {
            let store = self.store.lock();
            ids.into_iter()
               .filter_map(|id| store.agent(id).map(|agent| (id, agent.name.clone())))
               .collect::<Vec<_>>()
        };
        send_targets(agents.into_iter()
                           .map(|(id, name)| SendTarget { enabled: self.can_receive_prompt(id),
                                                          id,
                                                          name })
                           .collect())
    }

    /// Sends `item`'s prompt to agent `id` and nothing else. The view stays
    /// where it is: the user is triaging a list, not leaving it.
    pub(in crate::workspace_window) fn send_work_item(&mut self, id: Uuid, item: &WorkItemRef) -> bool {
        self.deliver_prompt(id, &item.prompt_text())
    }
}

/// Adds the "Send prompt to" item to `menu`: a submenu of `targets` when
/// there are any, else the item disabled.
pub(in crate::workspace_window) fn with_send_prompt(menu: PopupMenu, item: WorkItemRef,
                                                    targets: Vec<SendTarget>,
                                                    view: Entity<WorkspaceWindow>,
                                                    window: &mut Window,
                                                    cx: &mut Context<PopupMenu>)
                                                    -> PopupMenu {
    let title = knot_core::l10n::t("changes_view.send_prompt_to");
    if targets.is_empty() {
        return menu.item(PopupMenuItem::new(title).disabled(true));
    }
    menu.submenu(title, window, cx, move |mut submenu, _, _| {
            for target in &targets {
                let item = item.clone();
                let view = view.clone();
                let id = target.id;
                submenu = submenu.item(PopupMenuItem::new(target.name.clone())
                                           .disabled(!target.enabled)
                                           .on_click(move |_, _, app: &mut App| {
                                               view.update(app, |view, cx| {
                                                       view.send_work_item(id, &item);
                                                       cx.notify();
                                                   });
                                           }));
            }
            submenu
        })
}

#[cfg(test)]
mod tests;
