//! Inserting a library prompt from the `/` lookup: its text, with its
//! variables expanded for the agent whose composer it is, in place of the
//! slash token.
//!
//! Contract: `openspec/specs/panel-slash-commands/spec.md`, "Inserting a
//! library prompt expands its text".
//!
//! Expanding `{{branch}}` runs `git`, so the expansion runs on the
//! background executor and lands through `update_in`, which wakes the window
//! itself - no flag for `repaint_poll_tick` to poll. The buffer is captured
//! when the prompt is chosen and the text lands only if the buffer still
//! reads the same: an edit made while it resolved wins, and the expansion is
//! dropped.

use std::ops::Range;

use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;

impl WorkspaceWindow {
    /// Expand library prompt `prompt` for agent `id` and write it over
    /// `range` in `input` once it resolves.
    pub(in crate::workspace_window) fn expand_library_prompt(&mut self, id: Uuid, prompt: Uuid,
                                                             range: Range<usize>,
                                                             input: &Entity<PanelInputState>,
                                                             window: &mut Window,
                                                             cx: &mut Context<Self>) {
        let Some(text) = crate::settings_global::read(cx).prompt(prompt)
                                                         .map(|prompt| prompt.text.clone())
        else {
            return;
        };
        let Some(agent) = self.store.lock().agent(id).cloned()
        else {
            return;
        };
        let source = self.prompt_context_source(&agent);
        let snapshot = input.read(cx).value().to_string();
        let input = input.clone();
        cx.spawn_in(window, async move |this, cx| {
              let expanded =
                  cx.background_executor()
                    .spawn(async move {
                        knot_agent_launch::expand(&text, &knot_agent_launch::read_context(source))
                    })
                    .await;
              let _ = this.update_in(cx, |_, window, cx| {
                              apply_expansion(&input, &snapshot, range, &expanded, window, cx);
                              cx.notify();
                          });
          })
          .detach();
    }
}

/// Write `expanded` over `range` if `input` still holds `snapshot`.
fn apply_expansion(input: &Entity<PanelInputState>, snapshot: &str, range: Range<usize>,
                   expanded: &str, window: &mut Window, cx: &mut gpui_kit::App) {
    input.update(cx, |state, cx| {
             if state.value().as_str() != snapshot {
                 return;
             }
             state.set_selected_range(range, cx);
             state.replace(expanded, window, cx);
         });
}

#[cfg(test)]
mod tests;
