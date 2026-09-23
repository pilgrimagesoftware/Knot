//! Keeping each agent's composer styled: when the styling is created,
//! when it is recomputed, and what it reads to do so.
//!
//! The decisions live in [`crate::composer_style`]; this is the window
//! side - which agent, which attachment table, and which of the two
//! triggers fired.
//!
//! There are exactly two triggers, and they are deliberately different
//! shapes:
//!
//! - **The buffer changed.** `InputEvent::Change` fires for every way text
//!   arrives, so one subscription arm covers typing, paste, undo, redo, cut, a
//!   drag of text and the lookup's own insertion.
//! - **The appearance changed.** GPUI re-renders on an appearance switch, so
//!   the repaint hangs off the frame rather than off an observer of its own.
//!   Spans do not depend on the theme, so this repaints and never rescans.

use uuid::Uuid;

use crate::composer_style::ComposerStyling;
use crate::composer_style::Palette;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The attachment references for `id`, as the scanner wants them.
    ///
    /// Today an attachment's reference in the buffer is its path, which is
    /// what `send_panel_prompt` already appends to the prompt. Task group
    /// 7 gives it a shape of its own; this is the one place that has to
    /// change when it does.
    fn panel_attachment_refs(&self, id: Uuid) -> Vec<String> {
        self.panel_pending_context
            .get(&id)
            .map(|paths| {
                paths.iter()
                     .map(|path| path.to_string_lossy().into_owned())
                     .collect()
            })
            .unwrap_or_default()
    }

    /// Creates `id`'s styling if it has none, painting whatever the
    /// composer already holds.
    ///
    /// Called where the composer entity is created, so a restored draft is
    /// styled on arrival rather than on the user's first keystroke.
    pub(in crate::workspace_window) fn ensure_panel_styling(&mut self, id: Uuid,
                                                            palette: Palette,
                                                            cx: &mut gpui_kit::App) {
        if self.panel_composer_styling.contains_key(&id) {
            return;
        }
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        let refs = self.panel_attachment_refs(id);
        let borrowed: Vec<&str> = refs.iter().map(String::as_str).collect();
        let styling = ComposerStyling::new(&input, &borrowed, palette, cx);
        self.panel_composer_styling.insert(id, styling);
    }

    /// Restyles `id`'s composer after its buffer changed.
    pub(in crate::workspace_window) fn restyle_panel_composer(&mut self, id: Uuid,
                                                              palette: Palette,
                                                              cx: &mut gpui_kit::App) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        let refs = self.panel_attachment_refs(id);
        let borrowed: Vec<&str> = refs.iter().map(String::as_str).collect();
        let Some(styling) = self.panel_composer_styling.get_mut(&id)
        else {
            return;
        };
        styling.on_palette(palette, cx);
        styling.on_change(&input, &borrowed, cx);
    }

    /// Recomputes `id`'s styling from scratch, for a change to the
    /// attachment table rather than to the buffer.
    pub(in crate::workspace_window) fn restyle_panel_attachments(&mut self, id: Uuid,
                                                                 palette: Palette,
                                                                 cx: &mut gpui_kit::App) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        let refs = self.panel_attachment_refs(id);
        let borrowed: Vec<&str> = refs.iter().map(String::as_str).collect();
        let Some(styling) = self.panel_composer_styling.get_mut(&id)
        else {
            return;
        };
        styling.on_palette(palette, cx);
        styling.reset(&input, &borrowed, cx);
    }

    /// Repaints `id`'s composer if the appearance changed under it.
    ///
    /// Cheap enough for the render path on purpose: it compares a palette
    /// and returns, and only a real appearance switch does any work.
    pub(in crate::workspace_window) fn repaint_panel_styling_for_theme(&mut self, id: Uuid,
                                                                       palette: Palette,
                                                                       cx: &mut gpui_kit::App)
                                                                       -> bool {
        self.panel_composer_styling
            .get_mut(&id)
            .is_some_and(|styling| styling.on_palette(palette, cx))
    }
}
