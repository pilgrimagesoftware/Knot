use std::sync::Arc;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::component::WindowExt;
use gpui_kit::component::input::InputState;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts;
use crate::workspace_window::WorkspaceWindow;

mod render;

/// Whether a typed workspace name counts as nothing at all.
///
/// The confirm button's disabled state and the Return key both read this,
/// so a name the button refuses is a name Return refuses, by construction
/// rather than by two guards kept in step by hand. Judged on the trimmed
/// form, which is also the form `save_name` stores.
pub(crate) fn workspace_name_is_blank(name: &str) -> bool {
    name.trim().is_empty()
}

pub(crate) struct WorkspaceManager {
    pub(crate) store:                 Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) messages:              Arc<Mutex<knot_messaging::MessageStore>>,
    pub(crate) settings:              knot_core::Settings,
    pub(crate) name_input:            Entity<InputState>,
    pub(crate) editing_id:            Option<Uuid>,
    pub(crate) workspace_dialog_id:   Option<Uuid>,
    pub(crate) show_workspace_dialog: bool,
    pub(crate) error:                 Option<String>,
    /// Keeps the confirm button's disabled state honest while the user
    /// types: without it the button only re-reads the name on the next
    /// unrelated re-render, so an empty field could stay greyed after the
    /// first character.
    pub(crate) _name_subscription:    Subscription,
    pub(crate) _mcp_stop:             Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub(crate) struct WorkspaceDrag(Uuid);

pub(crate) struct WorkspaceDragPreview;

impl WorkspaceManager {
    fn persist(&mut self) {
        let store = self.store.lock();
        self.settings.saved_agents =
            store.saved_agents(self.settings.restore_conversation_on_launch);
        self.settings.saved_workspaces = store.saved_workspaces();
        if let Err(error) = self.settings.persist_roster() {
            self.error = Some(knot_core::l10n::t_with("workspace_manager.error_save",
                                                      &[("error", &error.to_string())]));
        }
    }

    fn save_name(&mut self, name: String, editing_id: Option<Uuid>, window: &mut Window,
                 cx: &mut Context<Self>) {
        if name.is_empty() {
            self.error = Some(knot_core::l10n::t("workspace_manager.error_name_empty"));
            cx.notify();
            return;
        }
        let mut store = self.store.lock();
        if let Some(id) = editing_id {
            if !store.rename_workspace(id, name) {
                self.error = Some(knot_core::l10n::t("workspace_manager.error_missing"));
                cx.notify();
                return;
            }
        }
        else {
            let id = Uuid::new_v4();
            store.add_workspace(knot_core::Workspace { id,
                                                       name,
                                                       color_hex: consts::COLOR_WORKSPACE_DEFAULT_HEX.to_string(),
                                                       agent_ids: Vec::new(),
                                                       layout_mode: "single".to_string(),
                                                       active_agent_ids: Vec::new(),
                                                       focused_pane_index: 0,
                                                       split_ratio: 0.5,
                                                       split_ratio_secondary: None,
                                                       show_dashboard: None,
                                                       is_detached: None,
                                                       window_bounds: None });
            store.set_current_workspace(id);
        }
        drop(store);
        self.persist();
        self.editing_id = None;
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.notify();
    }

    pub(crate) fn open_workspace_dialog(&mut self, editing_id: Option<Uuid>,
                                        window: &mut Window, cx: &mut Context<Self>) {
        let name = editing_id.and_then(|id| {
                                 let store = self.store.lock();
                                 store.workspaces()
                                      .iter()
                                      .find(|workspace| workspace.id == id)
                                      .map(|workspace| workspace.name.clone())
                             })
                             .unwrap_or_default();
        self.workspace_dialog_id = editing_id;
        self.show_workspace_dialog = true;
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.set_value(name, window, input_cx);
              input.focus(window, input_cx);
          });
        cx.notify();
    }

    fn cancel_workspace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace_dialog_id = None;
        self.show_workspace_dialog = false;
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.notify();
    }

    fn confirm_workspace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value().trim().to_string();
        let editing_id = self.workspace_dialog_id;
        self.save_name(name, editing_id, window, cx);
        if self.error.is_none() {
            self.workspace_dialog_id = None;
            self.show_workspace_dialog = false;
        }
        cx.notify();
    }

    fn delete(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.store.lock().remove_workspace(id) {
            self.error = Some(knot_core::l10n::t("workspace_manager.error_last_workspace"));
        }
        else {
            self.persist();
            self.error = None;
        }
        cx.notify();
    }

    /// Asks before deleting, through the shared alert dialog.
    ///
    /// `.claude/rules/knot-ui-conventions.md` requires `open_alert_dialog`
    /// for destructive actions, and this was the one window still painting
    /// its own overlay - so it was also the only confirmation without the
    /// dialog's focus trapping, Escape handling and default button.
    fn request_delete(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let name = self.store
                       .lock()
                       .workspaces()
                       .iter()
                       .find(|workspace| workspace.id == id)
                       .map(|workspace| workspace.name.clone())
                       .unwrap_or_else(|| "this workspace".to_string());
        let manager = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let manager = manager.clone();
                  alert.title(knot_core::l10n::t("workspace_manager.delete_title"))
                       .description(format!("Delete \"{name}\" and its agents?"))
                       .confirm()
                       .on_ok(move |_, _, app| {
                           manager.update(app, |view, cx| {
                                      view.delete(id, cx);
                                  });
                           true
                       })
              });
        cx.notify();
    }

    fn move_before(&mut self, id: Uuid, target_id: Uuid, cx: &mut Context<Self>) {
        if self.store.lock().move_workspace_before(id, target_id) {
            self.persist();
            cx.notify();
        }
    }

    fn select(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.store.lock().set_current_workspace(id);
        cx.notify();
    }

    fn open(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.select(id, cx);
        WorkspaceWindow::open(Arc::clone(&self.store),
                              Arc::clone(&self.messages),
                              self.settings.clone(),
                              id,
                              cx);
    }
}
