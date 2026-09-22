use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::switch::Switch;

use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    pub(crate) fn ai_provider_label(provider: &str) -> &'static str {
        match provider {
            "anthropic" => "Anthropic",
            "google" => "Google",
            _ => "OpenAI",
        }
    }

    /// Hardcoded model for each AI provider (cheapest/fastest options),
    /// matching the Swift reference's `AppSettings.aiModel(for:)`.
    pub(crate) fn ai_model_for(provider: &str) -> &'static str {
        match provider {
            "anthropic" => "claude-haiku-4-5",
            "google" => "gemini-flash-lite-latest",
            "openai" => "gpt-5-mini",
            _ => "",
        }
    }

    pub(crate) fn autopilot_action_label(action: &str) -> &'static str {
        match action {
            "ask" => "Ask me",
            "continue" => "Auto-continue",
            "custom" => "Custom",
            _ => "Mark conversation",
        }
    }

    pub(crate) fn autopilot_action_description(action: &str) -> &'static str {
        match action {
            "ask" => "Show a dialog letting you switch to the agent, dismiss, or auto-continue.",
            "continue" => "Automatically send \"yes, continue\" to the agent.",
            "custom" => {
                "Use your own prompt to decide what to reply. The LLM response is injected \
                 directly into the agent."
            }
            _ => "Set the agent status to indicate input is needed and send a notification.",
        }
    }

    fn select_ai_provider(&mut self, provider: &str, cx: &mut Context<Self>) {
        self.settings.ai_provider = provider.to_string();
        self.persist();
        cx.notify();
    }

    fn select_autopilot_action(&mut self, action: &str, cx: &mut Context<Self>) {
        self.settings.autopilot_action = action.to_string();
        self.persist();
        cx.notify();
    }

    pub(crate) fn save_ai_api_key(&mut self, cx: &mut Context<Self>) {
        self.settings.ai_api_key = self.ai_api_key_input.read(cx).value().to_string();
        self.persist();
    }

    pub(crate) fn save_autopilot_custom_prompt(&mut self, cx: &mut Context<Self>) {
        self.settings.autopilot_custom_prompt = self.autopilot_custom_prompt_input
                                                    .read(cx)
                                                    .value()
                                                    .to_string();
        self.persist();
    }

    pub(crate) fn render_autopilot(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let autopilot_enabled = self.settings.autopilot_enabled;
        let ai_provider = self.settings.ai_provider.clone();
        let autopilot_action = self.settings.autopilot_action.clone();
        let provider_label = Self::ai_provider_label(&ai_provider);
        let model_name = Self::ai_model_for(&ai_provider);
        let action_label = Self::autopilot_action_label(&autopilot_action);
        let is_custom_action = autopilot_action == "custom";

        v_flex()
            .gap_3()
            .child(
                Self::group("Enable")
                    .child(Self::row(
                        "Enable autopilot",
                        Switch::new("autopilot-enabled")
                            .checked(autopilot_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.autopilot_enabled = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::hint(
                        cx,
                        "Automatically detect when agents need input and take action — no need \
                         to babysit your agents. Only available with Claude Code.",
                    )),
            )
            .child(
                Self::group("AI Provider")
                    .child(Self::row(
                        "Provider",
                        Button::new("autopilot-provider-picker")
                            .label(provider_label)
                            .dropdown_caret(true)
                            .dropdown_menu({
                                let settings_window = settings_window.clone();
                                move |menu, _, _| {
                                    let mut menu = menu;
                                    for (label, value) in [
                                        ("OpenAI", "openai"),
                                        ("Anthropic", "anthropic"),
                                        ("Google", "google"),
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.select_ai_provider(value, cx);
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .child(Self::row(
                        "API Key",
                        Input::new(&self.ai_api_key_input)
                            .font_family(cx.theme().mono_font_family.clone())
                            .flex_1(),
                    ))
                    .child(Self::text_row(
                        "Model",
                        Self::mono_text(cx, model_name).text_color(cx.theme().muted_foreground),
                    )),
            )
            .child(
                Self::group("Action")
                    .child(Self::row(
                        "When input is detected",
                        Button::new("autopilot-action-picker")
                            .label(action_label)
                            .dropdown_caret(true)
                            .dropdown_menu({
                                let settings_window = settings_window.clone();
                                move |menu, _, _| {
                                    let mut menu = menu;
                                    for (label, value) in [
                                        ("Mark conversation", "mark"),
                                        ("Ask me", "ask"),
                                        ("Auto-continue", "continue"),
                                        ("Custom", "custom"),
                                    ] {
                                        menu = menu.item(PopupMenuItem::new(label).on_click({
                                            let settings_window = settings_window.clone();
                                            move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.select_autopilot_action(value, cx);
                                                })
                                            }
                                        }));
                                    }
                                    menu
                                }
                            }),
                    ))
                    .children(is_custom_action.then(|| {
                        Self::row(
                            "Custom prompt",
                            Input::new(&self.autopilot_custom_prompt_input).flex_1(),
                        )
                        .into_any_element()
                    }))
                    .children((!is_custom_action).then(|| {
                        Self::hint(cx, Self::autopilot_action_description(&autopilot_action))
                            .into_any_element()
                    })),
            )
    }
}
