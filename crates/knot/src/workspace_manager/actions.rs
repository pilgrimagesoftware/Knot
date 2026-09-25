//! What the workspace manager's rows and toolbar do: persisting the roster,
//! saving a name, deleting, reordering, selecting and opening.
//!
//! The name dialog's own three methods live in
//! [`crate::workspace_manager::dialog`], with the dialog they drive.

use std::sync::Arc;

use gpui_kit::AppContext;
use gpui_kit::Window;
use gpui_kit::component::WindowExt;
use gpui_kit::{App, Context};
use uuid::Uuid;

use crate::consts;
use crate::workspace_manager::WorkspaceManager;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceManager {
    /// Writes the roster. Every caller has just changed the workspace list,
    /// so the menu bar's Select Workspace submenu is brought up to date too.
    pub(super) fn persist(&mut self, cx: &mut App) {
        // Scoped so the store guard is released before the file write: the
        // same blocking-I/O rule the workspace window's persist follows.
        let installed = {
            let store = self.store.lock();
            crate::settings_global::write(cx, |settings| {
                settings.saved_agents = store.saved_agents(settings.restore_conversation_on_launch);
                settings.saved_workspaces = store.saved_workspaces();
            })
        };
        if let Err(error) = installed.persist_roster() {
            self.error = Some(knot_core::l10n::t_with("workspace_manager.error_save",
                                                      &[("error", &error.to_string())]));
        }
        crate::menu_bar::refresh_menu_bar_workspaces(&self.store, cx);
    }

    pub(super) fn save_name(&mut self, name: String, editing_id: Option<Uuid>,
                            window: &mut Window, cx: &mut Context<Self>) {
        if name.is_empty() {
            self.error = Some(knot_core::l10n::t("workspace_manager.error_name_empty"));
            cx.notify();
            return;
        }
        let mut store = self.store.lock();
        let renamed = editing_id.is_some();
        if let Some(id) = editing_id {
            if !store.rename_workspace(id, name) {
                self.error = Some(knot_core::l10n::t("workspace_manager.error_missing"));
                cx.notify();
                return;
            }
        }
        else {
            let id = Uuid::new_v4();
            store.add_workspace(knot_core::Workspace { id,name,color_hex: consts::COLOR_WORKSPACE_DEFAULT_HEX.to_string(),agent_ids: Vec::new() });
            store.set_current_workspace(id);
        }
        drop(store);
        self.persist(cx);
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.notify();
        if renamed {
            // The renamed workspace's own window draws its title from this
            // shared store, and `cx.notify()` marks only *this* window dirty
            // - so without a scheduled paint over there it keeps showing the
            // name it last drew. Reading live state does not cause a paint;
            // being scheduled for one does. `import_window/window.rs`'s
            // `redraw_every_window` is the same fix for the same class of
            // staleness.
            cx.refresh_windows();
        }
    }

    fn delete(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.store.lock().remove_workspace(id) {
            self.error = Some(knot_core::l10n::t("workspace_manager.error_last_workspace"));
        }
        else {
            self.persist(cx);
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
    pub(super) fn request_delete(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        self.error = None;
        let name = self.store
                       .lock()
                       .workspaces()
                       .iter()
                       .find(|workspace| workspace.id == id)
                       .map(|workspace| workspace.name.clone())
                       .unwrap_or_else(|| knot_core::l10n::t("workspace_manager.this_workspace"));
        let manager = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let manager = manager.clone();
                  alert.title(knot_core::l10n::t("workspace_manager.delete_title"))
                       .description(knot_core::l10n::t_with("workspace_manager.delete_body",
                                                            &[("name", &name)]))
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

    pub(super) fn move_before(&mut self, id: Uuid, target_id: Uuid, cx: &mut Context<Self>) {
        if self.store.lock().move_workspace_before(id, target_id) {
            self.persist(cx);
            cx.notify();
        }
    }

    pub(super) fn select(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.store.lock().set_current_workspace(id);
        cx.notify();
    }

    pub(super) fn open(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.select(id, cx);
        WorkspaceWindow::open(Arc::clone(&self.store), Arc::clone(&self.messages), id, cx);
    }
}
