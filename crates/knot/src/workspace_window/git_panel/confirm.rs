//! The confirmation in front of discard.
//!
//! Discard is the one operation in the panel that nothing can undo: it throws
//! away work that was never committed, in a view whose whole purpose is
//! reviewing work an agent did that the user has not read yet. Swift fired it
//! on a single click of a hover-revealed icon; the port asks first.
//!
//! Staging and unstaging are deliberately not confirmed - each is undone by
//! its opposite, so a prompt there would be friction with nothing behind it.

use std::path::PathBuf;

use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::{Context, Window};
use uuid::Uuid;

use super::actions::GitAction;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Asks before discarding, naming the path, and does nothing if the user
    /// cancels.
    pub(super) fn confirm_discard(&mut self, id: Uuid, folder: &str, paths: Vec<PathBuf>,
                                  display: &str, window: &mut Window, cx: &mut Context<Self>) {
        let title = knot_core::l10n::t_with("git_panel.discard_title", &[("path", display)]);
        let body = knot_core::l10n::t_with("git_panel.discard_body", &[("path", display)]);
        let folder = folder.to_string();
        let entity = cx.entity();

        window.open_alert_dialog(cx, move |alert, _, _| {
                  let entity = entity.clone();
                  let folder = folder.clone();
                  let paths = paths.clone();
                  alert.title(title.clone())
                       .description(body.clone())
                       // Named and tinted rather than a bare "OK": the action
                       // destroys work nothing can recover, so the button
                       // says what it does.
                       .button_props(DialogButtonProps::default()
                           .ok_text(knot_core::l10n::t("git_panel.discard_confirm"))
                           .ok_variant(ButtonVariant::Danger)
                           .show_cancel(true))
                       .on_ok(move |_, _, app| {
                           entity.update(app, |view, cx| {
                                     view.run_git_action(id,
                                                         &folder,
                                                         GitAction::Discard,
                                                         paths.clone(),
                                                         cx);
                                 });
                           true
                       })
              });
    }
}
