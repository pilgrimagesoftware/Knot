//! The editor's two native pickers - the agent's folder and its avatar - and
//! the one-grapheme rule the avatar field is held to.

use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::PathPromptOptions;
use gpui_kit::Window;
use gpui_kit::component::input::InputState;
use unicode_segmentation::UnicodeSegmentation;

use super::window::AgentEditor;
// macOS-only: the module it names is `cfg(target_os = "macos")`, and so
// is every use of it here.
#[cfg(target_os = "macos")]
use crate::app_support::native_character_picker;

impl AgentEditor {
    pub(super) fn choose_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(knot_core::l10n::t("agent_editor.choose_folder_prompt").into()),
        });
        let editor = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              cx.update(|app| {
                    editor.update(app, |editor, cx| {
                              editor.folder_path = path.to_string_lossy().into_owned();
                              cx.notify();
                          });
                });
          })
          .detach();
    }

    /// Clears the avatar field, then focuses it and opens the OS character
    /// picker - it inserts the chosen character into whatever field has
    /// keyboard focus, so clearing first makes the picker replace the
    /// current avatar rather than append to it.
    pub(super) fn choose_avatar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.avatar_input.update(cx, |input, cx| {
                             input.set_value("", window, cx);
                             input.focus(window, cx);
                         });
        #[cfg(target_os = "macos")]
        native_character_picker::open();
    }

    /// Keeps the avatar field to a single character (grapheme cluster), so
    /// typing or pasting past one character doesn't silently grow it.
    pub(super) fn clamp_avatar_to_one_character(&mut self, avatar_input: &Entity<InputState>,
                                                window: &mut Window, cx: &mut Context<Self>) {
        let value = avatar_input.read(cx).value().to_string();
        let Some(first) = value.graphemes(true).next()
        else {
            return;
        };
        if first.len() == value.len() {
            return;
        }
        let first = first.to_string();
        avatar_input.update(cx, |input, cx| {
                        input.set_value(first, window, cx);
                    });
    }
}
