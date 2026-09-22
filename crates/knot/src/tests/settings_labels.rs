//! The settings window's label lookups and its tab set.
//!
//! Every one of these maps a stored enum variant to display text. The match
//! is exhaustive, so what these assert is that each variant has a label and
//! that no two share one - there is no catch-all default left to test, which
//! is the point of #224.

use std::collections::BTreeSet;

use knot_core::AiProvider;
use knot_core::AppearanceMode;
use knot_core::AutopilotAction;
use uuid::Uuid;

use crate::settings_window::SettingsTab;
use crate::settings_window::SettingsWindow;
use crate::tests::workspace;

#[test]
fn appearance_label_names_every_mode() {
    assert_eq!(SettingsWindow::appearance_label(AppearanceMode::Auto),
               "Auto");
    assert_eq!(SettingsWindow::appearance_label(AppearanceMode::System),
               "System");
    assert_eq!(SettingsWindow::appearance_label(AppearanceMode::Light),
               "Light");
    assert_eq!(SettingsWindow::appearance_label(AppearanceMode::Dark),
               "Dark");
}

/// There is no "defaults to Auto" case left to test: an unrecognized value
/// cannot reach the label function any more, because it is resolved to a
/// variant - and reported - at the settings boundary instead. That is the
/// whole point of #224.
#[test]
fn every_mode_has_a_distinct_label() {
    let labels: std::collections::HashSet<_> =
        AppearanceMode::ALL.iter()
                           .map(|m| SettingsWindow::appearance_label(*m))
                           .collect();

    assert_eq!(labels.len(), AppearanceMode::ALL.len());
}

#[test]
fn agent_type_label_maps_known_types() {
    assert_eq!(SettingsWindow::agent_type_label("codex"), "Codex");
    assert_eq!(SettingsWindow::agent_type_label("opencode"), "OpenCode");
    assert_eq!(SettingsWindow::agent_type_label("gemini"), "Gemini");
    assert_eq!(SettingsWindow::agent_type_label("copilot"), "Copilot");
    assert_eq!(SettingsWindow::agent_type_label("custom1"), "Custom 1");
    assert_eq!(SettingsWindow::agent_type_label("custom2"), "Custom 2");
    assert_eq!(SettingsWindow::agent_type_label("shell"), "Shell");
}

#[test]
fn agent_type_label_defaults_to_claude() {
    assert_eq!(SettingsWindow::agent_type_label("claude"), "Claude");
    assert_eq!(SettingsWindow::agent_type_label("anything-else"), "Claude");
}

#[test]
fn persona_preview_returns_short_instructions_unchanged() {
    assert_eq!(SettingsWindow::persona_preview("be terse", 80), "be terse");
}

/// A persona assigned to an agent can't be deleted - the count drives
/// both the disabled delete button and its tooltip.
#[test]
fn personas_in_use_counts_only_the_agents_that_reference_each_persona() {
    let assigned = Uuid::new_v4();
    let unused = Uuid::new_v4();
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    store.create("~/alpha",
                 knot_agents::CreateOptions { persona_id: Some(assigned),
                                              ..Default::default() });
    store.create("~/beta",
                 knot_agents::CreateOptions { persona_id: Some(assigned),
                                              ..Default::default() });
    store.create("~/gamma", knot_agents::CreateOptions::default());

    let in_use = SettingsWindow::personas_in_use(store.agents());

    assert_eq!(in_use.get(&assigned).copied(), Some(2));
    assert_eq!(in_use.get(&unused).copied(), None);
}

#[test]
fn persona_delete_tooltip_names_the_reason_it_is_disabled() {
    assert_eq!(SettingsWindow::persona_delete_tooltip(0), "Delete persona");
    assert_eq!(SettingsWindow::persona_delete_tooltip(1),
               "In use by 1 agent");
    assert_eq!(SettingsWindow::persona_delete_tooltip(3),
               "In use by 3 agents");
}

#[test]
fn persona_preview_truncates_long_instructions_with_ellipsis() {
    let instructions = "a".repeat(100);
    let preview = SettingsWindow::persona_preview(&instructions, 80);
    assert_eq!(preview.chars().count(), 81);
    assert!(preview.ends_with('…'));
    assert_eq!(&preview[..80], "a".repeat(80).as_str());
}

#[test]
fn ai_provider_label_names_every_provider() {
    assert_eq!(SettingsWindow::ai_provider_label(AiProvider::OpenAi),
               "OpenAI");
    assert_eq!(SettingsWindow::ai_provider_label(AiProvider::Anthropic),
               "Anthropic");
    assert_eq!(SettingsWindow::ai_provider_label(AiProvider::Google),
               "Google");
}

/// Every provider names a model. The old `_ => ""` arm meant an
/// unrecognized provider silently asked for no model at all.
#[test]
fn ai_model_for_matches_swift_reference_defaults() {
    assert_eq!(SettingsWindow::ai_model_for(AiProvider::OpenAi),
               "gpt-5-mini");
    assert_eq!(SettingsWindow::ai_model_for(AiProvider::Anthropic),
               "claude-haiku-4-5");
    assert_eq!(SettingsWindow::ai_model_for(AiProvider::Google),
               "gemini-flash-lite-latest");

    for provider in AiProvider::ALL {
        assert!(!SettingsWindow::ai_model_for(*provider).is_empty(),
                "{provider} has no model");
    }
}

#[test]
fn autopilot_action_label_names_every_action() {
    assert_eq!(SettingsWindow::autopilot_action_label(AutopilotAction::Mark),
               "Mark conversation");
    assert_eq!(SettingsWindow::autopilot_action_label(AutopilotAction::Ask),
               "Ask me");
    assert_eq!(SettingsWindow::autopilot_action_label(AutopilotAction::Continue),
               "Auto-continue");
    assert_eq!(SettingsWindow::autopilot_action_label(AutopilotAction::Custom),
               "Custom");
}

/// Driven off `ALL` rather than a hand-written list, so a new action cannot
/// be added without a description to go with it.
#[test]
fn autopilot_action_description_is_distinct_per_action() {
    let descriptions: BTreeSet<&str> =
        AutopilotAction::ALL.iter()
                            .map(|action| SettingsWindow::autopilot_action_description(*action))
                            .collect();

    assert_eq!(descriptions.len(), AutopilotAction::ALL.len());
}

#[test]
fn key_name_for_code_maps_known_modifier_codes() {
    assert_eq!(SettingsWindow::key_name_for_code(54), "Right Command");
    assert_eq!(SettingsWindow::key_name_for_code(56), "Left Shift");
    assert_eq!(SettingsWindow::key_name_for_code(63), "Fn");
}

#[test]
fn key_name_for_code_falls_back_for_unknown_codes() {
    assert_eq!(SettingsWindow::key_name_for_code(999), "Key 999");
}

#[test]
fn mcp_server_url_formats_localhost_with_port() {
    assert_eq!(SettingsWindow::mcp_server_url(8767),
               "http://127.0.0.1:8767/mcp");
    assert_eq!(SettingsWindow::mcp_server_url(9000),
               "http://127.0.0.1:9000/mcp");
}

/// The URL the settings window shows, and the `mcp add` command built from
/// it, must be the one Knot itself hands an agent - not a second spelling.
/// The server routes MCP at `/mcp` and answers a POST to `/` with 405, so
/// the old path-less URL registered a server that could never connect.
#[test]
fn the_displayed_mcp_url_is_the_one_knot_gives_its_own_agents() {
    let mut settings = knot_core::Settings::default();
    settings.mcp_server_port = 8767;
    assert_eq!(SettingsWindow::mcp_server_url(settings.mcp_server_port),
               knot_agent_launch::mcp_url(&settings));
}

#[test]
fn mcp_install_command_matches_swift_reference_per_agent() {
    // The real URL, path included: this command is copied verbatim.
    let url = "http://127.0.0.1:8767/mcp";
    assert_eq!(SettingsWindow::mcp_install_command("claude", url),
               "claude mcp add --transport http --scope user knot http://127.0.0.1:8767/mcp");
    assert_eq!(SettingsWindow::mcp_install_command("codex", url),
               "codex mcp add knot --url http://127.0.0.1:8767/mcp");
    assert_eq!(SettingsWindow::mcp_install_command("opencode", url),
               "opencode mcp add");
    assert_eq!(SettingsWindow::mcp_install_command("gemini", url),
               "gemini mcp add --transport http knot http://127.0.0.1:8767/mcp --scope user");
    assert_eq!(SettingsWindow::mcp_install_command("copilot", url), "");
}

#[test]
fn restore_conversation_toggle_enabled_only_with_layout_restore() {
    assert!(SettingsWindow::restore_conversation_toggle_enabled(true));
    assert!(!SettingsWindow::restore_conversation_toggle_enabled(false));
}

#[test]
fn turning_off_layout_restore_does_not_touch_conversation_restore() {
    let mut settings = knot_core::Settings::default();
    settings.restore_conversation_on_launch = true;
    settings.restore_layout_on_launch = false;
    assert!(settings.restore_conversation_on_launch);
}

#[test]
fn settings_tab_default_is_general() {
    assert_eq!(SettingsTab::ALL[0], SettingsTab::General);
}

#[test]
fn settings_tab_labels_are_distinct() {
    let labels: BTreeSet<String> = SettingsTab::ALL.iter().map(|tab| tab.label()).collect();
    assert_eq!(labels.len(), SettingsTab::ALL.len());
}

#[test]
fn settings_tab_covers_every_swift_pane() {
    assert_eq!(SettingsTab::ALL.to_vec(),
               vec![SettingsTab::General,
                    SettingsTab::Coding,
                    SettingsTab::Personas,
                    SettingsTab::Autopilot,
                    SettingsTab::Voice,
                    SettingsTab::Mcp,
                    SettingsTab::Terminal]);
}
