//! The editor's Startup Prompt control, over the shared picker in
//! [`crate::startup_choice`].
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
use gpui_kit::div;
use knot_core::StartupPrompt;

use super::AgentEditor;
use crate::startup_choice::{self, StartupChoice};

/// What the editor submits: nothing for a `shell` agent, which has no coding
/// agent to prompt - so switching an agent to `shell` clears it - and the
/// picker's choice otherwise.
pub(super) fn submitted_startup_prompt(choice: StartupChoice, custom_text: &str,
                                       agent_type: &str)
                                       -> Option<StartupPrompt> {
    if knot_core::agent_type::is_shell(agent_type) {
        return None;
    }
    startup_choice::submitted(choice, custom_text)
}

impl AgentEditor {
    /// The Startup Prompt rows, or nothing for a `shell` agent.
    pub(super) fn startup_prompt_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        if knot_core::agent_type::is_shell(&self.agent_type) {
            return Vec::new();
        }
        let library = crate::settings_global::read(cx).prompts.clone();
        let editor = cx.entity();
        let picker = startup_choice::picker("agent-startup-prompt-picker",
                                            self.startup_choice,
                                            library,
                                            move |choice, app| {
                                                editor.update(app, |e, cx| {
                                                          e.startup_choice = choice;
                                                          cx.notify();
                                                      })
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
    fn a_shell_agent_submits_no_startup_prompt() {
        assert_eq!(submitted_startup_prompt(StartupChoice::Custom, "go", "claude"),
                   StartupPrompt::custom("go"));
        assert_eq!(submitted_startup_prompt(StartupChoice::Custom, "go", "shell"),
                   None,
                   "switching an agent to shell clears its startup prompt on submit");
    }
}
