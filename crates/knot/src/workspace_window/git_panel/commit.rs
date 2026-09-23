//! Opening the commit window and running what it asks for.
//!
//! The window itself is `crate::commit_window`; this is the panel's side of
//! it - the handler that takes the message, runs `git commit` off the render
//! path, and reports back through the slot the window polls.

use gpui_kit::{Context, Window};
use uuid::Uuid;

use crate::commit_window::{CommitOutcome, open_commit_window};
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Opens the commit window for `id`.
    ///
    /// The window owns the message and stays open until the commit succeeds,
    /// so a rejecting hook does not cost the user what they typed.
    pub(super) fn open_commit_window(&mut self, id: Uuid, folder: &str, _window: &mut Window,
                                     cx: &mut Context<Self>) {
        let folder = folder.to_string();
        let entity = cx.entity();

        cx.defer(move |cx| {
              open_commit_window(move |message: String,
                                       outcome: CommitOutcome,
                                       _window: &mut Window,
                                       app: &mut gpui_kit::App| {
                                     entity.update(app, |view, _cx| {
                                               view.commit_git_panel(id, &folder, &message,
                                                                     outcome);
                                           });
                                 },
                                 cx);
          });
    }

    /// Invalidates the panel after a commit landed, and resumes its watch.
    ///
    /// Called from the repaint poll: the commit runs on the blocking pool and
    /// reports into a slot the commit window polls, but the *panel* has to be
    /// told too - the window closing is not what refreshes the tree behind
    /// it.
    pub(super) fn finish_commit(&mut self, id: Uuid) {
        self.resume_git_watch(id);
        self.invalidate_git_state(id);
    }
}
