//! The Startup Prompt picker both editors show - the agent editor's and the
//! Settings window's bench-entry editor's - and what each choice submits as.
//!
//! Contract: `openspec/specs/agent-editor-ui/spec.md`, "Startup prompt
//! control", and `openspec/specs/settings-ui/spec.md`, "Bench tab".

use gpui_kit::App;
use gpui_kit::SharedString;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use knot_core::{Prompt, StartupPrompt};
use uuid::Uuid;

/// Which form of startup prompt a picker has selected. Custom text lives in
/// the editor's own text field, so switching away from Custom and back does
/// not lose it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StartupChoice {
    None,
    Library(Uuid),
    Custom,
}

/// The choice and custom text a stored startup prompt opens as.
pub(crate) fn initial_choice(startup: Option<&StartupPrompt>) -> (StartupChoice, String) {
    match startup {
        None => (StartupChoice::None, String::new()),
        Some(StartupPrompt::Library(id)) => (StartupChoice::Library(*id), String::new()),
        Some(StartupPrompt::Custom(text)) => (StartupChoice::Custom, text.clone()),
    }
}

/// What a picker submits: the chosen form, with blank custom text as none.
/// A library reference is submitted as it stands, dangling or not - leaving
/// the picker untouched keeps it.
pub(crate) fn submitted(choice: StartupChoice, custom_text: &str) -> Option<StartupPrompt> {
    match choice {
        StartupChoice::None => None,
        StartupChoice::Library(id) => Some(StartupPrompt::Library(id)),
        StartupChoice::Custom => StartupPrompt::custom(custom_text),
    }
}

/// A choice's label: a library prompt by name, or "Missing prompt" for one no
/// longer in the library - shown as missing rather than as None, so the user
/// can see why nothing is sent.
pub(crate) fn choice_label(choice: StartupChoice, library: &[Prompt]) -> String {
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

/// The picker: a dropdown showing `current`, offering None, each library
/// prompt by name, and Custom, calling `on_pick` with the choice made.
pub(crate) fn picker(id: impl Into<SharedString>, current: StartupChoice, library: Vec<Prompt>,
                     on_pick: impl Fn(StartupChoice, &mut App) + Clone + 'static)
                     -> impl gpui_kit::IntoElement {
    Button::new(id.into()).label(choice_label(current, &library))
                          .dropdown_caret(true)
                          .dropdown_menu(move |menu, _, _| {
                              let mut menu = menu.scrollable(true);
                              let choices =
                                  [StartupChoice::None].into_iter()
                                                       .chain(library.iter()
                                                                     .map(|prompt| {
                                                                         StartupChoice::Library(prompt.id)
                                                                     }))
                                                       .chain([StartupChoice::Custom]);
                              for choice in choices {
                                  let on_pick = on_pick.clone();
                                  menu = menu.item(PopupMenuItem::new(choice_label(choice,
                                                                                   &library))
                                                       .on_click(move |_, _, app| on_pick(choice, app)));
                              }
                              menu
                          })
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
    fn submission_follows_the_choice() {
        let id = Uuid::new_v4();
        assert_eq!(submitted(StartupChoice::None, "x"), None);
        assert_eq!(submitted(StartupChoice::Library(id), ""),
                   Some(StartupPrompt::Library(id)));
        assert_eq!(submitted(StartupChoice::Custom, "go"),
                   StartupPrompt::custom("go"));
        assert_eq!(submitted(StartupChoice::Custom, "  "), None);
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
