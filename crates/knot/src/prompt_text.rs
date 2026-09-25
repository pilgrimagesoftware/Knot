//! What every field that edits prompt text offers: the list of prompt
//! variables to insert at the caret, and a warning naming the ones it does
//! not recognise.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "Prompt variables" and
//! "Unknown variables and escaping". Shared by the agent editor's custom
//! startup prompt and the Settings window's prompt editor, so the two cannot
//! offer different lists or warn differently.

use gpui_kit::App;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::menu::PopupMenu;
use gpui_kit::component::menu::PopupMenuItem;
use knot_agent_launch::PromptVariable;

/// The catalogue key describing what `variable` expands to.
fn description_key(variable: PromptVariable) -> &'static str {
    match variable {
        PromptVariable::AgentName => "prompt_variables.agent_name",
        PromptVariable::AgentId => "prompt_variables.agent_id",
        PromptVariable::AgentType => "prompt_variables.agent_type",
        PromptVariable::Folder => "prompt_variables.folder",
        PromptVariable::FolderName => "prompt_variables.folder_name",
        PromptVariable::Workspace => "prompt_variables.workspace",
        PromptVariable::Branch => "prompt_variables.branch",
        PromptVariable::Date => "prompt_variables.date",
    }
}

/// A variable's menu label: the placeholder and what it expands to.
pub(crate) fn variable_label(variable: PromptVariable) -> String {
    format!("{}  {}",
            variable.placeholder(),
            knot_core::l10n::t(description_key(variable)))
}

/// `menu` with one item per prompt variable, each inserting its placeholder
/// into `target` at the caret.
pub(crate) fn variable_menu(mut menu: PopupMenu, target: &Entity<TextareaState>) -> PopupMenu {
    for variable in PromptVariable::ALL {
        let target = target.clone();
        menu = menu.item(PopupMenuItem::new(variable_label(variable)).on_click(
            move |_, window, app| insert_variable(&target, variable, window, app),
        ));
    }
    menu
}

/// Write `variable`'s placeholder at `target`'s caret, replacing any
/// selection, the way typing it would.
pub(crate) fn insert_variable(target: &Entity<TextareaState>, variable: PromptVariable,
                              window: &mut Window, app: &mut App) {
    target.update(app, |state, cx| {
              state.replace(variable.placeholder(), window, cx);
          });
}

/// The warning to show under prompt text naming the variables it does not
/// recognise, or `None` when every `{{...}}` in it is a known variable.
/// A warning, never an error: the text still saves and still sends, with
/// the unknown names left as typed.
pub(crate) fn unknown_variables_warning(text: &str) -> Option<String> {
    let unknown = knot_agent_launch::unknown_variables(text);
    (!unknown.is_empty()).then(|| {
                             knot_core::l10n::t_with("prompt_variables.unknown",
                                                     &[("names", &unknown.join(", "))])
                         })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variable_has_a_description_that_resolves() {
        for variable in PromptVariable::ALL {
            let key = description_key(variable);
            assert_ne!(knot_core::l10n::t(key), key, "{key} must resolve");
            assert!(variable_label(variable).starts_with(&variable.placeholder()));
        }
    }

    #[test]
    fn the_warning_names_each_unknown_variable() {
        assert_eq!(unknown_variables_warning("{{folder}} and \\{{x}}"), None);
        let warning = unknown_variables_warning("{{foldr}} {{brnach}}").expect("two unknown names");
        assert_ne!(warning, "prompt_variables.unknown", "the key must resolve");
        assert!(warning.contains("foldr") && warning.contains("brnach"),
                "{warning}");
    }
}
