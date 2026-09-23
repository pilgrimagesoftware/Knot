//! An attachment's second presence: the reference token in the buffer,
//! and keeping it in step with the row in the pending-context table.
//!
//! The table is what gets **sent**; the token is how the user sees and
//! moves the attachment (`acp-panel-ui`, "Attached context SHALL have two
//! presences"). They are reconciled in one direction only - **the buffer
//! decides**. A token the user deleted detaches its row, and removing a
//! row deletes its token as an ordinary edit that flows back through the
//! same reconciliation.
//!
//! One rule rather than two-way syncing, because two-way syncing has to
//! arbitrate a conflict it can never actually resolve: if the buffer says
//! one thing and the table another, only one of them is what the user
//! just did, and only the buffer knows which.
//!
//! Insertion is deferred to the next frame. Two of the three ways context
//! arrives have a window in hand and one - the add-context control, which
//! completes asynchronously after the file picker closes - does not, and
//! editing the buffer needs one. A single deferred path beats two that
//! work and one that is special.

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use gpui_kit::Entity;
use gpui_kit::Window;
use uuid::Uuid;

use crate::composer_scan::escape_token;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;

#[cfg(test)]
mod tests;

/// The text that stands for `path` in the buffer.
///
/// The file's own name rather than its full path: a chip is read at a
/// glance, and the absolute path still reaches the agent on the `Attached:`
/// line `send_panel_prompt` appends. Escaped for the same reason a mention
/// is - a name containing a space has to stay one token.
pub(in crate::workspace_window) fn attachment_reference(path: &Path) -> String {
    let name = path.file_name()
                   .map(|name| name.to_string_lossy().into_owned())
                   .unwrap_or_else(|| path.to_string_lossy().into_owned());
    format!("@{}", escape_token(&name))
}

impl WorkspaceWindow {
    /// Queues a reference to `path` for insertion into `id`'s composer.
    ///
    /// Called by every path that attaches context, so all three produce
    /// both presences.
    pub(in crate::workspace_window) fn queue_attachment_reference(&mut self, id: Uuid,
                                                                  path: PathBuf) {
        self.panel_pending_attachments
            .entry(id)
            .or_default()
            .push(path);
    }

    /// Inserts the references queued for `id`, if any.
    ///
    /// Called from the panel's render, which is the first place after an
    /// attachment arrives that has a window to edit the buffer with.
    pub(in crate::workspace_window) fn insert_queued_attachments(&mut self, id: Uuid,
                                                                 window: &mut Window,
                                                                 cx: &mut gpui_kit::App)
                                                                 -> bool {
        let queued = self.panel_pending_attachments
                         .remove(&id)
                         .unwrap_or_default();
        if queued.is_empty() {
            return false;
        }
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return false;
        };
        for path in &queued {
            insert_reference(&input, &attachment_reference(path), window, cx);
        }
        true
    }

    /// Drops every table row whose reference is no longer in the buffer.
    ///
    /// Returns whether anything was detached. Rows still waiting to be
    /// inserted are left alone: their token is not in the buffer yet, and
    /// reading that as a deletion would detach an attachment the moment it
    /// arrived.
    pub(in crate::workspace_window) fn reconcile_panel_attachments(&mut self, id: Uuid,
                                                                   cx: &gpui_kit::App)
                                                                   -> bool {
        let Some(input) = self.panel_prompt_inputs.get(&id)
        else {
            return false;
        };
        let text = input.read(cx).value().to_string();
        let queued = self.panel_pending_attachments
                         .get(&id)
                         .cloned()
                         .unwrap_or_default();
        let Some(paths) = self.panel_pending_context.get_mut(&id)
        else {
            return false;
        };

        let kept = surviving_attachments(&text, paths, &queued);

        let detached = kept.len() != paths.len();
        *paths = kept;
        if detached && paths.is_empty() {
            self.panel_pending_context.remove(&id);
        }
        detached
    }

    /// Deletes `path`'s reference from `id`'s buffer, for a row dismissed
    /// from the strip.
    ///
    /// A programmatic edit, so it reports as an ordinary change and the
    /// reconciliation above drops the row - the strip does not remove it
    /// directly. One direction, one rule.
    pub(in crate::workspace_window) fn remove_attachment_reference(&mut self, id: Uuid,
                                                                   path: &Path,
                                                                   window: &mut Window,
                                                                   cx: &mut gpui_kit::App)
                                                                   -> bool {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return false;
        };
        let reference = attachment_reference(path);
        let text = input.read(cx).value().to_string();
        // The last occurrence, so dismissing one of two identical chips
        // removes one of them rather than always the first.
        let Some(start) = text.rfind(&reference)
        else {
            return false;
        };
        let end = start + reference.len();
        // The separating space that insertion added goes with it, so
        // removing a chip does not leave a double space behind.
        let start = if text[..start].ends_with(' ') {
            start - 1
        }
        else {
            start
        };

        input.update(cx, |state, cx| {
                 state.set_selected_range(start..end, cx);
                 state.replace("", window, cx);
             });
        true
    }
}

/// The rows of `paths` whose reference is still in `text`.
///
/// Counted rather than merely looked for: two attachments whose names are
/// the same produce the same reference, so with two identical chips and
/// one deleted exactly one row has to survive. Looking for the reference
/// would keep both, and dropping on a single miss would lose both.
///
/// A row in `queued` is waiting for its chip to be written and is kept
/// regardless - its token is legitimately not there yet, and reading that
/// as a deletion would detach an attachment the instant it arrived.
pub(in crate::workspace_window) fn surviving_attachments(text: &str, paths: &[PathBuf],
                                                         queued: &[PathBuf])
                                                         -> Vec<PathBuf> {
    let mut used: BTreeMap<String, usize> = BTreeMap::new();
    let mut kept: Vec<PathBuf> = Vec::new();
    for path in paths {
        if queued.contains(path) {
            kept.push(path.clone());
            continue;
        }
        let reference = attachment_reference(path);
        let occurrences = text.matches(&reference).count();
        let seen = used.entry(reference).or_insert(0);
        if *seen < occurrences {
            *seen += 1;
            kept.push(path.clone());
        }
    }
    kept
}

/// What insertion writes at the caret, given the text before it.
///
/// A separating space where the character before the caret is not already
/// one: the `@` has to sit at a word boundary or the scanner reads it as
/// prose, which would leave the chip unstyled and the reconciliation
/// unable to find it.
pub(in crate::workspace_window) fn insertion_text(before_caret: &str, reference: &str) -> String {
    let needs_space = before_caret.chars()
                                  .next_back()
                                  .is_some_and(|before| !before.is_whitespace());
    if needs_space {
        format!(" {reference}")
    }
    else {
        reference.to_string()
    }
}

/// Writes `reference` at the caret, with a separating space where the
/// character before it is not already one.
///
/// The `@` has to sit at a word boundary or the scanner reads it as prose,
/// which would leave the chip unstyled and the reconciliation unable to
/// find it.
fn insert_reference(input: &Entity<PanelInputState>, reference: &str, window: &mut Window,
                    cx: &mut gpui_kit::App) {
    input.update(cx, |state, cx| {
             let caret = state.cursor();
             let inserted = insertion_text(&state.value()[..caret], reference);
             state.set_selected_range(caret..caret, cx);
             state.replace(&inserted, window, cx);
         });
}
