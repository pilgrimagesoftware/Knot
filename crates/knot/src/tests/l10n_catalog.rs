//! Catalog coverage for the strings that would otherwise ship as their own
//! keys - a missing entry renders the key, which is visible but easy to
//! miss in review.

use crate::app_state::AgentListBackgroundEntry;
use crate::app_state::AgentMenuEntry;
use crate::app_state::SidebarMenuFacts;
use crate::app_state::sidebar_background_menu_entries;
use crate::settings_window::SettingsTab;
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
                "settings.personas.edit",
                "settings.personas.add",
                "settings.personas.delete_body",
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

/// The agent editor, the workspace manager and the broadcast sheet had no
/// catalog references at all before #223. Harvested from the source, so a
/// label added without an entry fails rather than shipping its key.
#[test]
fn dialog_labels_resolve() {
    for key in ["agent_editor.activation",
                "agent_editor.activation_hint",
                "agent_editor.avatar",
                "agent_editor.cancel",
                "agent_editor.capabilities",
                "agent_editor.capabilities_hint",
                "agent_editor.capabilities_placeholder",
                "agent_editor.coding_agent",
                "agent_editor.command",
                "agent_editor.cost_tier",
                "agent_editor.cost_tier_high",
                "agent_editor.cost_tier_hint",
                "agent_editor.cost_tier_low",
                "agent_editor.cost_tier_medium",
                "agent_editor.description",
                "agent_editor.description_placeholder",
                "agent_editor.error_choose_folder",
                "agent_editor.no_folder",
                "agent_editor.persona_none",
                "agent_editor.error_enter_name",
                "agent_editor.folder",
                "agent_editor.name",
                "agent_editor.persona",
                "agent_editor.shell_command",
                "agent_editor.title_edit",
                "agent_editor.title_new",
                "broadcast.cancel",
                "broadcast.placeholder",
                "broadcast.send",
                "workspace_manager.cancel",
                "workspace_manager.delete",
                "workspace_manager.delete_title",
                "workspace_manager.error_last_workspace",
                "workspace_manager.error_missing",
                "workspace_manager.error_name_empty",
                "workspace_manager.new",
                "workspace_manager.open",
                "workspace_manager.rename",
                "workspace_manager.title",
                "workspace_manager.workspace"]
    {
        let value = knot_core::l10n::t(key);
        assert_ne!(value, key, "{key} is missing from the catalog");
        assert!(!value.is_empty(), "{key} resolves to an empty string");
    }
}

/// Driven by `ALL` rather than a written-out key list, so a variant added
/// without a catalog entry fails here instead of drawing its own key in the
/// menu. Separators have no label and are skipped.
#[test]
fn every_menu_entry_label_resolves() {
    for entry in AgentMenuEntry::ALL {
        let Some(label) = entry.label()
        else {
            assert_eq!(entry, AgentMenuEntry::Separator);
            continue;
        };
        assert!(!label.starts_with("menu.agent."),
                "{entry:?} is missing from the catalog: {label}");
    }
}

/// The sidebar's background menu, the same way. It has no `ALL`, so the
/// entries come from a full menu - which this one always is, since it
/// disables rather than omits.
#[test]
fn every_sidebar_menu_entry_label_resolves() {
    for item in sidebar_background_menu_entries(SidebarMenuFacts::default()) {
        let Some(label) = item.entry.label()
        else {
            assert_eq!(item.entry, AgentListBackgroundEntry::Separator);
            continue;
        };
        assert!(!label.starts_with("menu.sidebar."),
                "{:?} is missing from the catalog: {label}",
                item.entry);
    }
}

/// The slash lookup's own copy: the popup's key hint, and the description
/// beside every built-in command. A missing key ships the key itself as the
/// line explaining what a command does.
#[test]
fn slash_lookup_labels_resolve() {
    for key in ["panel.lookup_hint",
                "panel.command.broadcast",
                "panel.command.check",
                "panel.command.create_agent",
                "panel.command.list_agents",
                "panel.command.list_repos",
                "panel.command.list_worktrees",
                "panel.command.send",
                "panel.command.show_markdown",
                "panel.command.worktree"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// Every settings tab's title, so an added pane cannot show `settings.tabs.*`
/// where its name belongs.
#[test]
fn every_settings_tab_label_resolves() {
    for tab in SettingsTab::ALL {
        let label = tab.label();
        assert!(!label.starts_with("settings.tabs."),
                "{tab:?} is missing from the catalog: {label}");
    }
}

/// The Import window draws every word it shows from the catalog, its title
/// and empty states included - and an empty state is exactly the copy a
/// missing key would replace with a key string, since it only appears when
/// there is nothing else on screen to notice.
#[test]
fn import_window_labels_resolve() {
    for key in ["import.title",
                "import.personas_title",
                "import.personas_none",
                "import.workspaces_title",
                "import.workspaces_none",
                "import.import_selected",
                "import.import_count",
                "import.result_title",
                "import.nothing_to_do",
                "import.refresh"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The three failures a panel writes into the conversation itself. Each
/// embeds the underlying error, so a body that lost its placeholder would
/// report a failure without saying what failed.
#[test]
fn panel_transcript_errors_resolve_and_keep_their_cause() {
    for key in ["panel.error_first_turn",
                "panel.error_registration",
                "panel.error_answer"]
    {
        let message = knot_core::l10n::t_with(key, &[("error", "connection refused")]);
        assert_ne!(message, key, "{key} is missing from the catalog");
        assert!(message.contains("connection refused"),
                "{key} must carry the cause: {message}");
        assert!(!message.contains("%{error}"),
                "{key} left its placeholder unfilled");
    }
}

/// The chrome a panel draws around a conversation - two decision buttons,
/// four icon-only controls whose tooltip is their only name, and the two
/// lines a panel shows when it has no conversation yet.
#[test]
fn panel_chrome_labels_resolve() {
    for key in ["panel.allow",
                "panel.deny",
                "panel.close",
                "panel.stop",
                "panel.running",
                "panel.connecting",
                "panel.select_agent_to_start",
                "panel.attach_context",
                "panel.scroll_to_latest",
                "panel.scroll_to_your_message",
                "panel.scroll_to_top"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The four confirmations, and the values they embed. A body that lost its
/// placeholder would ask "Restart ?" and still pass a resolution check, so
/// these assert the substitution as well as the key.
#[test]
fn menu_confirmation_copy_resolves_and_keeps_its_values() {
    for key in ["menu.agent.confirm.restart_title",
                "menu.agent.confirm.remove_title",
                "menu.sidebar.confirm.restart_all_title",
                "menu.sidebar.confirm.close_all_title"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }

    for key in ["menu.agent.confirm.restart_body",
                "menu.agent.confirm.remove_body"]
    {
        let body = knot_core::l10n::t_with(key, &[("name", "Alpha")]);
        assert!(body.contains("Alpha"), "{key} must name the agent: {body}");
        assert!(!body.contains("%{name}"),
                "{key} left its placeholder unfilled: {body}");
    }

    for key in ["menu.sidebar.confirm.restart_all_body",
                "menu.sidebar.confirm.close_all_body"]
    {
        let agents = knot_core::l10n::pluralize(3, "count.agent", "count.agents");
        let body = knot_core::l10n::t_with(key, &[("agents", &agents)]);
        assert!(body.contains("3 agents"), "{key} must say how many: {body}");
        assert!(!body.contains("%{agents}"),
                "{key} left its placeholder unfilled: {body}");
    }
}

/// The window title and the one error the sidebar can show, neither of
/// which is reachable from a label lookup.
/// The Window menu's own items. A missing key here ships the key string as a
/// menu label, in the one menu a user goes to when they have lost a window.
#[test]
fn window_menu_labels_resolve() {
    for key in ["menu.window.command_center", "menu.window.workspaces"] {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn window_chrome_labels_resolve() {
    for key in ["broadcast.window_title",
                "workspace.name_placeholder",
                "workspace.missing",
                "workspace.choose_agent"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }

    let error = knot_core::l10n::t_with("sidebar.width_error", &[("error", "disk full")]);
    assert!(error.contains("disk full"),
            "the width error must say what failed: {error}");
    assert!(!error.contains("%{error}"));
}
