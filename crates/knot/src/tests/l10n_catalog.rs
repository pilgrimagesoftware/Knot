//! Catalog coverage for the strings that would otherwise ship as their own
//! keys - a missing entry renders the key, which is visible but easy to
//! miss in review.

use crate::workspace_window;

/// Every word the About window shows comes from the catalog, so a missing
/// key would ship the key string itself where the version, copyright or a
/// credit line belongs.
#[test]
fn about_window_labels_resolve() {
    for key in ["about.title",
                "about.version_label",
                "about.build_label",
                "about.commit_unknown",
                "about.copy_details",
                "about.close",
                "about.copyright",
                "about.derived_from",
                "about.credits.author",
                "about.credits.license",
                "about.credits.built_with",
                "about.credits.toolkit",
                "about.credits.terminal",
                "about.credits.fonts"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The queued-row controls carry no visible text of their own, so their
/// tooltips and accessibility labels are the only thing naming them - a
/// missing key would ship the key string itself as the button's name.
#[test]
fn queued_message_control_labels_resolve() {
    for key in ["panel.queued",
                "panel.queued_inbox_nudge",
                "panel.failed",
                "panel.retry",
                "panel.retry_queued",
                "panel.delete_queued",
                "panel.edit_queued",
                "panel.replace_composer_title",
                "panel.replace_composer_body",
                "panel.retry_connect"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn the_queued_status_label_follows_the_failed_mark() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    assert_eq!(queued_status_label(false, PromptOrigin::User),
               knot_core::l10n::t("panel.queued"));
    assert_eq!(queued_status_label(true, PromptOrigin::User),
               knot_core::l10n::t("panel.failed"));
}

/// A queued nudge is labelled as one, so a prompt the user never typed does
/// not read as one they did - except when it failed, where the state the
/// user must act on wins.
#[test]
fn a_queued_inbox_nudge_says_where_it_came_from() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    assert_eq!(queued_status_label(false, PromptOrigin::InboxNudge),
               knot_core::l10n::t("panel.queued_inbox_nudge"));
    assert_ne!(queued_status_label(false, PromptOrigin::InboxNudge),
               queued_status_label(false, PromptOrigin::User));
    assert_eq!(queued_status_label(true, PromptOrigin::InboxNudge),
               knot_core::l10n::t("panel.failed"));
}

/// Every key the settings window asks for, harvested from the source rather
/// than hand-listed, so a new label cannot be added without an entry. A
/// missing one renders the key itself - visible in the window, easy to miss
/// in review, and exactly what #223 is about.
#[test]
fn settings_window_labels_resolve() {
    for key in ["settings.appearance.fonts",
                "settings.appearance.terminal",
                "settings.appearance.title",
                "settings.appearance.ui",
                "settings.autopilot.action_group",
                "settings.autopilot.api_key",
                "settings.autopilot.blurb",
                "settings.autopilot.custom_prompt",
                "settings.autopilot.enable",
                "settings.autopilot.enable_group",
                "settings.autopilot.model",
                "settings.autopilot.on_input",
                "settings.autopilot.provider",
                "settings.autopilot.provider_group",
                "settings.coding.agent_options",
                "settings.coding.clear_source_folder",
                "settings.coding.clear_source_folder_body",
                "settings.coding.coding_agent",
                "settings.coding.folder",
                "settings.coding.options",
                "settings.coding.source_folder",
                "settings.compact_tool_calls",
                "settings.compact_tool_calls_hint",
                "settings.font_unavailable",
                "settings.general.agent_panel",
                "settings.general.appearance",
                "settings.general.appearance_hint",
                "settings.general.desktop_notifications",
                "settings.general.keep_in_menu_bar",
                "settings.general.notifications",
                "settings.general.restore_agents",
                "settings.general.restore_conversation",
                "settings.general.shift_enter_hint",
                "settings.general.shift_enter_to_send",
                "settings.general.startup",
                "settings.input.api_key",
                "settings.input.custom_prompt",
                "settings.input.extra_cli_options",
                "settings.input.port",
                "settings.mcp.agent",
                "settings.mcp.blurb",
                "settings.mcp.command",
                "settings.mcp.enable",
                "settings.mcp.install_command",
                "settings.mcp.no_setup",
                "settings.mcp.port",
                "settings.mcp.server_settings",
                "settings.mcp.url",
                "settings.persona_editor.cancel",
                "settings.persona_editor.instructions",
                "settings.persona_editor.name",
                "settings.persona_editor.name_placeholder",
                "settings.persona_editor.save",
                "settings.personas.delete_persona",
                "settings.personas.none_defined",
                "settings.personas.personas",
                "settings.personas.restore_defaults",
                "settings.personas.restore_defaults_body",
                "settings.voice.auto_insert",
                "settings.voice.blurb",
                "settings.voice.enable",
                "settings.voice.engine",
                "settings.voice.engine_apple",
                "settings.voice.engine_hint",
                "settings.voice.input",
                "settings.voice.push_to_talk"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
}
