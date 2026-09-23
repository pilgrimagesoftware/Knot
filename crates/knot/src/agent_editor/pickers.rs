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
    pub(super) fn choose_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(knot_core::l10n::t("agent_editor.choose_folder_prompt").into()),
        });
        // `spawn_in` rather than `spawn`: naming the agent after the folder
        // means writing an `InputState`, which needs a window, and the plain
        // app context a `spawn` closure gets has none.
        cx.spawn_in(window, async move |this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              let _ = this.update_in(cx, |editor, window, cx| {
                              editor.folder_path = path.to_string_lossy().into_owned();
                              editor.name_the_agent_after_its_folder(window, cx);
                              cx.notify();
                          });
          })
          .detach();
    }

    /// Puts the chosen folder's name in the name field, if the field is
    /// empty.
    ///
    /// The field is read here rather than before the picker opened, so a name
    /// typed while the picker was up wins: by the time this runs the field is
    /// no longer blank, and a name the user supplied is never replaced.
    ///
    /// A folder with no last component - a root, or a path ending in `..` -
    /// leaves the field alone rather than filling it with nothing. The user
    /// keeps the blank field they had and the ordinary "enter a name" error.
    ///
    /// What counts as blank, and what a folder is worth naming an agent, are
    /// [`super::fields::name_from_folder`]'s to decide - it needs no entity,
    /// so it is tested on its own. This applies the answer.
    fn name_the_agent_after_its_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.name_input.read(cx).value().to_string();
        let Some(name) = super::fields::name_from_folder(&current, &self.folder_path)
        else {
            return;
        };
        self.name_input.update(cx, |input, cx| {
                           input.set_value(name, window, cx);
                       });
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
