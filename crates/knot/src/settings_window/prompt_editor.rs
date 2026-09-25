//! The prompt add/edit window the Prompts tab opens.
//!
//! Contract: `openspec/specs/settings-ui/spec.md`, "Prompts tab". Laid out
//! like the persona editor beside it: a name field and a multi-line text
//! field, with the variable list and unknown-variable warning from
//! [`crate::prompt_text`] under the text, and Create or Save disabled while
//! either field is blank.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use crate::app_support::observe_system_appearance;
use crate::settings_window::SettingsWindow;
use crate::settings_window::persona_editor::persona_editor_window_options;

/// Opens the prompt editor: `prompt` is `None` for "Add Prompt…" and `Some`
/// for a row's edit action.
pub(crate) fn open_prompt_editor(parent: WeakEntity<SettingsWindow>,
                                 prompt: Option<knot_core::Prompt>, cx: &mut App) {
    let editing_id = prompt.as_ref().map(|p| p.id);
    let title = if editing_id.is_some() {
        knot_core::l10n::t("settings.prompt_editor.title_edit")
    }
    else {
        knot_core::l10n::t("settings.prompt_editor.title_new")
    };
    let name = prompt.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    let text = prompt.as_ref().map(|p| p.text.clone()).unwrap_or_default();
    let options = persona_editor_window_options(title, cx);
    let _ = cx.open_window(options, move |window, cx| {
                  observe_system_appearance(window);
                  let name_input = cx.new(|cx| {
                                         InputState::new(window, cx)
                            .placeholder(knot_core::l10n::t("settings.prompt_editor.name_placeholder"))
                            .default_value(name)
                                     });
                  name_input.update(cx, |state, cx| state.focus(window, cx));
                  let text_input = cx.new(|cx| {
                                         TextareaState::new(window, cx)
                            .placeholder(knot_core::l10n::t("settings.prompt_editor.text_placeholder"))
                            .default_value(text)
                                     });
                  let view = cx.new(|cx| {
                                   // Both fields gate the save button and the text
                                   // drives the warning, so each edit re-renders.
                                   let subscriptions =
                                       vec![cx.subscribe(&name_input, repaint_on_change),
                                            cx.subscribe(&text_input, repaint_on_change)];
                                   PromptEditor { parent,
                                                  editing_id,
                                                  name_input,
                                                  text_input,
                                                  _subscriptions: subscriptions }
                               });
                  cx.new(|cx| Root::new(view, window, cx))
              });
}

fn repaint_on_change<E>(_: &mut PromptEditor, _: Entity<E>, event: &InputEvent,
                        cx: &mut Context<PromptEditor>) {
    if matches!(event, InputEvent::Change) {
        cx.notify();
    }
}

pub(crate) struct PromptEditor {
    parent:         WeakEntity<SettingsWindow>,
    editing_id:     Option<Uuid>,
    name_input:     Entity<InputState>,
    text_input:     Entity<TextareaState>,
    _subscriptions: Vec<Subscription>,
}

impl PromptEditor {
    fn fields(&self, cx: &App) -> (String, String) {
        (self.name_input.read(cx).value().to_string(), self.text_input.read(cx).value().to_string())
    }

    /// Whether both fields hold something other than whitespace - the same
    /// rule `knot_core::Prompt::new` refuses a prompt by.
    fn can_save(&self, cx: &App) -> bool {
        let (name, text) = self.fields(cx);
        !name.trim().is_empty() && !text.trim().is_empty()
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_save(cx) {
            return;
        }
        let (name, text) = self.fields(cx);
        let name = name.trim().to_string();
        let result =
            crate::settings_global::write_persisting(cx, |settings| match self.editing_id {
                Some(id) => settings.update_prompt(id, name, text),
                None => settings.add_prompt(name, text).map(|_| ()),
            });
        if let Err(error) = result {
            eprintln!("failed to save the prompt: {error}");
        }
        if let Some(parent) = self.parent.upgrade() {
            parent.update(cx, |_, cx| cx.notify());
        }
        window.remove_window();
    }
}

impl Render for PromptEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (_, text) = self.fields(cx);
        let warning = crate::prompt_text::unknown_variables_warning(&text);
        let target = self.text_input.clone();
        let insert = Button::new("prompt-editor-variables")
            .label(knot_core::l10n::t("prompt_variables.insert"))
            .dropdown_caret(true)
            .dropdown_menu(move |menu, _, _| crate::prompt_text::variable_menu(menu, &target));
        let label = |key: &str| {
            div().w(px(100.))
                 .text_right()
                 .child(knot_core::l10n::t(key))
        };
        v_flex()
            .size_full()
            .gap_3()
            .p_5()
            .bg(cx.theme().background)
            .child(h_flex().gap_2()
                           .child(label("settings.prompt_editor.name"))
                           .child(Input::new(&self.name_input).flex_1()))
            .child(h_flex().flex_1()
                           .gap_2()
                           .child(label("settings.prompt_editor.text"))
                           .child(v_flex().flex_1()
                                          .h_full()
                                          .gap_1()
                                          .child(Textarea::new(&self.text_input).flex_1()
                                                                                .h_full()
                                                                                .font_family(cx.theme()
                                                                                               .mono_font_family
                                                                                               .clone()))
                                          .child(div().child(insert))
                                          .children(warning.map(|warning| {
                                                               div().text_xs()
                                                                    .text_color(cx.theme().warning)
                                                                    .child(warning)
                                                           }))))
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-prompt-editor")
                            .label(knot_core::l10n::t("settings.prompt_editor.cancel"))
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("save-prompt-editor")
                            .label(knot_core::l10n::t(if self.editing_id.is_some() {
                                "settings.prompt_editor.save"
                            } else {
                                "settings.prompt_editor.create"
                            }))
                            .primary()
                            .disabled(!self.can_save(cx))
                            .on_click(cx.listener(|editor, _, window, cx| editor.save(window, cx))),
                    ),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
