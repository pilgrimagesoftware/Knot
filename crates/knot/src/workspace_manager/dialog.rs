//! The dialog that names a workspace, for a new one and for a rename.
//!
//! It is hosted by `window.open_dialog` rather than painted here. The window
//! used to build it as a `div().absolute()` overlay with its own key
//! listener, which meant it was also the one dialog in the crate without the
//! focus trapping, Escape handling and stacking every other one inherits -
//! the same gap `.claude/rules/knot-ui-conventions.md` closes for the
//! destructive confirmations by requiring `open_alert_dialog`. The delete
//! confirmation next to it already went through the shared host; this one
//! now does too (`openspec/specs/workspace-manager-ui`, issue #225).
//!
//! `open_alert_dialog` itself is not the vehicle: an `AlertDialog` is a
//! title, a description and two buttons, with nowhere to put the name field.
//! `open_dialog` is the same host underneath - overlay, focus trap, Escape,
//! layer stacking - with a content area this can put an `Input` in.
//!
//! Nothing the dialog draws is read from the [`WorkspaceManager`] entity.
//! The dialog layer is rendered by `root_overlays` at the end of this
//! window's own `Render`, so the builder below runs *inside* that entity's
//! update - reading it there panics with "cannot read while it is already
//! being updated". So the title comes from a flag captured when the dialog
//! opened, and the confirm button's disabled state from the name input,
//! which is its own entity and is not mid-update. The callbacks may touch
//! the manager freely: they run from a click or a keystroke, not a paint.
//!
//! Return and Escape are no longer this module's business either. The dialog
//! host binds them to `Confirm` and `Cancel` in its own key context, and the
//! focused `Input` propagates both, so the keys arrive at [`Dialog::on_ok`]
//! and [`Dialog::on_cancel`] - the same two callbacks the footer buttons
//! reach. One path per outcome, rather than a key listener and a click
//! handler kept in step by hand.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::dialog::Cancel;
use gpui_kit::component::dialog::Confirm;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::px;
use uuid::Uuid;

use crate::consts;
use crate::workspace_manager::WorkspaceManager;
use crate::workspace_manager::workspace_name_is_blank;

impl WorkspaceManager {
    /// Opens the name dialog, on an existing workspace to rename it or on
    /// `None` to create one.
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
        self.error = None;
        let manager = cx.entity();
        let input = self.name_input.clone();
        let renaming = editing_id.is_some();
        window.open_dialog(cx, move |dialog, _window, app| {
                  Self::build_dialog(dialog, renaming, &input, &manager, app)
              });
        // After `open_dialog`, not before: opening focuses the dialog's own
        // handle, so a field focused first would lose it immediately. The
        // convention is that a dialog autofocuses its first meaningful
        // input, and here that is the only input.
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.set_value(name, window, input_cx);
              input.focus(window, input_cx);
          });
        cx.notify();
    }

    /// Rebuilt every frame by the dialog layer, which is what lets the
    /// confirm button's disabled state follow what the user has typed
    /// without this module holding a second copy of the name.
    ///
    /// `renaming` is passed in rather than read back off `manager`: see the
    /// module docs for why nothing here may read that entity.
    fn build_dialog(dialog: Dialog, renaming: bool, input: &Entity<InputState>,
                    manager: &Entity<WorkspaceManager>, app: &mut App)
                    -> Dialog {
        let name_is_blank = workspace_name_is_blank(&input.read(app).value());

        dialog.w(px(consts::WORKSPACE_DIALOG_WIDTH))
              // No X: the footer's Cancel is the way out, as it was before
              // the dialog moved into the shared host.
              .close_button(false)
              // A stray click on the backdrop should not discard a half-typed
              // name. Escape still cancels, deliberately.
              .overlay_closable(false)
              .title(knot_core::l10n::t(if renaming {
                                            "workspace_manager.rename_title"
                                        }
                                        else {
                                            "workspace_manager.new_title"
                                        }))
              .content({
                  let input = input.clone();
                  move |content, _window, _app| content.child(Input::new(&input))
              })
              .footer(Self::dialog_footer(renaming, name_is_blank))
              .on_ok({
                  let manager = manager.clone();
                  move |_, window, app| Self::confirm_from_dialog(&manager, window, app)
              })
              .on_cancel({
                  let manager = manager.clone();
                  move |_, window, app| {
                      manager.update(app, |view, cx| {
                                 view.cancel_workspace_dialog(window, cx);
                             });
                      true
                  }
              })
    }

    /// Cancel and confirm, dispatched rather than wired straight to the two
    /// methods: the dialog host owns closing, so both buttons raise the same
    /// actions Escape and Return do and let [`Dialog::on_ok`] /
    /// [`Dialog::on_cancel`] decide. A button that called the methods itself
    /// would have to close the dialog too, and would then be the one path
    /// that could close it twice.
    fn dialog_footer(renaming: bool, name_is_blank: bool) -> impl IntoElement {
        DialogFooter::new()
            .child(Button::new("cancel-workspace-dialog").label(knot_core::l10n::t("workspace_manager.cancel"))
                                                         .on_click(|_: &ClickEvent, window, app| {
                                                             window.dispatch_action(Box::new(Cancel), app);
                                                         }))
            .child(Button::new("confirm-workspace-dialog").label(knot_core::l10n::t(if renaming {
                                                              "workspace_manager.save"
                                                          }
                                                          else {
                                                              "workspace_manager.create"
                                                          }))
                                                          .primary()
                                                          .disabled(name_is_blank)
                                                          .on_click(|_: &ClickEvent, window, app| {
                                                              window.dispatch_action(Box::new(Confirm { secondary: false }), app);
                                                          }))
    }

    /// Answers the dialog's "may I close?": `false` leaves it open.
    ///
    /// A blank name is inert rather than an error, matching the disabled
    /// confirm button - Return on an empty field should do nothing, not
    /// scold. A name that fails to save (the workspace was deleted from
    /// under the rename) leaves the dialog open, with the window's error
    /// line behind it saying why.
    fn confirm_from_dialog(manager: &Entity<WorkspaceManager>, window: &mut Window,
                           app: &mut App)
                           -> bool {
        let input = manager.read(app).name_input.clone();
        if workspace_name_is_blank(&input.read(app).value()) {
            return false;
        }
        manager.update(app, |view, cx| {
                   view.confirm_workspace_dialog(window, cx);
               });
        manager.read(app).error.is_none()
    }

    fn cancel_workspace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace_dialog_id = None;
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
        }
        cx.notify();
    }
}
