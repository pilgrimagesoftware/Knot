//! The editor's Startup Prompt control: None, a library prompt, or Custom
//! text, and what each submits as.
//!
//! Contract: `openspec/specs/agent-editor-ui/spec.md`, "Startup prompt
//! control".

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::div;
use knot_core::{Prompt, StartupPrompt};
use uuid::Uuid;

use super::AgentEditor;

/// Which form of startup prompt the control has selected. The custom text
/// itself lives in the editor's text field, so switching away from Custom
/// and back does not lose it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartupChoice {
    None,
    Library(Uuid),
    Custom,
}

/// The choice and custom text an agent's stored startup prompt opens as.
pub(super) fn initial_choice(startup: Option<&StartupPrompt>) -> (StartupChoice, String) {
    match startup {
        None => (StartupChoice::None, String::new()),
        Some(StartupPrompt::Library(id)) => (StartupChoice::Library(*id), String::new()),
        Some(StartupPrompt::Custom(text)) => (StartupChoice::Custom, text.clone()),
    }
}

/// What the form submits: nothing for a `shell` agent, which has no coding
/// agent to prompt; otherwise the chosen form, with blank custom text as
/// none. A library reference is submitted as it stands, dangling or not -
/// leaving the control untouched keeps it.
pub(super) fn submitted_startup_prompt(choice: StartupChoice, custom_text: &str,
                                       agent_type: &str)
                                       -> Option<StartupPrompt> {
    if knot_core::agent_type::is_shell(agent_type) {
        return None;
    }
    match choice {
        StartupChoice::None => None,
        StartupChoice::Library(id) => Some(StartupPrompt::Library(id)),
        StartupChoice::Custom => StartupPrompt::custom(custom_text),
    }
}

/// The control's label for `choice`: a library prompt by name, or "Missing
/// prompt" for one no longer in the library - shown as missing rather than
/// as None, so the user can see why nothing is sent.
pub(super) fn choice_label(choice: StartupChoice, library: &[Prompt]) -> String {
    match choice {
        StartupChoice::None => knot_core::l10n::t("agent_editor.startup_prompt_none"),
        StartupChoice::Custom => knot_core::l10n::t("agent_editor.startup_prompt_custom"),
        StartupChoice::Library(id) => {
            library.iter()
                   .find(|prompt| prompt.id == id)
                   .map(|prompt| prompt.name.clone())
                   .unwrap_or_else(|| knot_core::l10n::t("agent_editor.startup_prompt_missing"))
        }
    }
}

impl AgentEditor {
    /// The Startup Prompt rows, or nothing for a `shell` agent.
    pub(super) fn startup_prompt_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        if knot_core::agent_type::is_shell(&self.agent_type) {
            return Vec::new();
        }
        let library = crate::settings_global::read(cx).prompts.clone();
        let editor = cx.entity();
        let picker = Button::new("agent-startup-prompt-picker")
            .label(choice_label(self.startup_choice, &library))
            .dropdown_caret(true)
            .dropdown_menu(move |menu, _, _| {
                let mut menu = menu.scrollable(true);
                let choices = [StartupChoice::None].into_iter()
                                                   .chain(library.iter()
                                                                 .map(|p| StartupChoice::Library(p.id)))
                                                   .chain([StartupChoice::Custom]);
                for choice in choices {
                    let editor = editor.clone();
                    menu = menu.item(PopupMenuItem::new(choice_label(choice, &library)).on_click(
                        move |_, _, app| {
                            editor.update(app, |e, cx| {
                                      e.startup_choice = choice;
                                      cx.notify();
                                  })
                        },
                    ));
                }
                menu
            });
        let mut rows = vec![Self::dialog_row(knot_core::l10n::t("agent_editor.startup_prompt"),
                                             picker).into_any_element()];
        if self.startup_choice == StartupChoice::Custom {
            rows.push(self.custom_startup_prompt(cx).into_any_element());
        }
        rows.push(Self::dialog_hint(cx, knot_core::l10n::t("agent_editor.startup_prompt_hint"))
                      .into_any_element());
        rows
    }

    /// What the form submits for the startup prompt, per
    /// [`submitted_startup_prompt`].
    pub(super) fn submitted_startup_prompt(&self, cx: &gpui_kit::App) -> Option<StartupPrompt> {
        let text = self.startup_custom_input.read(cx).value().to_string();
        submitted_startup_prompt(self.startup_choice, &text, &self.agent_type)
    }

    /// The custom text field, the variable list that inserts into it, and
    /// the warning naming any variable it does not recognise.
    fn custom_startup_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let text = self.startup_custom_input.read(cx).value().to_string();
        let warning = crate::prompt_text::unknown_variables_warning(&text);
        let target = self.startup_custom_input.clone();
        let insert = Button::new("agent-startup-prompt-variables")
            .label(knot_core::l10n::t("prompt_variables.insert"))
            .dropdown_caret(true)
            .dropdown_menu(move |menu, _, _| crate::prompt_text::variable_menu(menu, &target));
        v_flex().gap_1()
                .child(Textarea::new(&self.startup_custom_input).h(gpui_kit::px(96.)))
                .child(div().child(insert))
                .children(warning.map(|warning| {
                                     div().text_xs()
                                          .text_color(cx.theme().warning)
                                          .child(warning)
                                 }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stored_prompt_opens_as_its_choice() {
        let id = Uuid::new_v4();
        assert_eq!(initial_choice(None), (StartupChoice::None, String::new()));
        assert_eq!(initial_choice(Some(&StartupPrompt::Library(id))).0,
                   StartupChoice::Library(id));
        assert_eq!(initial_choice(Some(&StartupPrompt::Custom("go".into()))),
                   (StartupChoice::Custom, "go".to_string()));
    }

    #[test]
    fn submission_follows_the_choice_and_the_type() {
        let id = Uuid::new_v4();
        assert_eq!(submitted_startup_prompt(StartupChoice::None, "x", "claude"),
                   None);
        assert_eq!(submitted_startup_prompt(StartupChoice::Library(id), "", "claude"),
                   Some(StartupPrompt::Library(id)));
        assert_eq!(submitted_startup_prompt(StartupChoice::Custom, "go", "claude"),
                   StartupPrompt::custom("go"));
        assert_eq!(submitted_startup_prompt(StartupChoice::Custom, "  ", "claude"),
                   None);
        assert_eq!(submitted_startup_prompt(StartupChoice::Custom, "go", "shell"),
                   None,
                   "a shell agent's startup prompt is cleared on submit");
    }

    #[test]
    fn a_dangling_reference_is_labelled_missing_not_none() {
        let library = vec![Prompt::new("gate", "make").unwrap()];
        let missing = choice_label(StartupChoice::Library(Uuid::new_v4()), &library);
        assert_eq!(missing,
                   knot_core::l10n::t("agent_editor.startup_prompt_missing"));
        assert_ne!(missing, choice_label(StartupChoice::None, &library));
        assert_eq!(choice_label(StartupChoice::Library(library[0].id), &library),
                   "gate");
    }
}
