//! Guards the `#[serde(rename_all = "camelCase")]` alignment: a document in the
//! Swift `CodingKeys` shape must load field-for-field, and re-serializing it
//! must reproduce the same keys.
//!
//! The Swift shape is one document holding everything, which is also the
//! legacy shape the store migrates off, so these fixtures are written as
//! `settings.json` and read back through the migration.

use knot_core::consts::{
    AGENTS_FILE, LEGACY_SETTINGS_FILE, PERSONAS_FILE, PREFERENCES_FILE, WORKSPACES_FILE,
};
use knot_core::{
    AiProvider, AppearanceMode, AutopilotAction, CostTier, PersonaState, PersonaType, Settings,
};
use uuid::Uuid;

const FIXTURE: &str = include_str!("fixtures/settings_swift_shape.json");

/// Write `document` as the legacy single document in a fresh store.
fn legacy_store(document: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(LEGACY_SETTINGS_FILE), document).unwrap();
    dir
}

fn read_json(path: std::path::PathBuf) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

#[test]
fn loads_swift_shaped_document() {
    let dir = legacy_store(FIXTURE);

    let s = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(s.appearance_mode, AppearanceMode::Dark);
    assert!(!s.restore_layout_on_launch);
    assert!(!s.restore_conversation_on_launch);
    assert!(s.keep_in_menu_bar);
    assert!(!s.mcp_server_enabled);
    assert_eq!(s.mcp_server_port, 9123);
    assert_eq!(s.source_base_folder, "~/code");
    assert!(s.source_folder_detected);
    assert!(!s.desktop_notifications_enabled);
    assert_eq!(s.markdown_font_size, 16);
    assert_eq!(s.mermaid_theme, "forest");
    assert_eq!(s.mermaid_scale, 1.5);
    assert_eq!(s.agent_commands.get("custom1").map(String::as_str),
               Some("my-agent"));
    assert_eq!(s.agent_options.get("claude").map(String::as_str),
               Some("--verbose"));
    assert_eq!(s.terminal_font_name, "Menlo");
    assert_eq!(s.terminal_font_size, 12.5);

    // Fixture predates the autopilot scalars; decode-tolerant defaults apply.
    assert!(!s.autopilot_enabled);
    assert_eq!(s.ai_provider, AiProvider::OpenAi);
    assert_eq!(s.ai_api_key, "");
    assert_eq!(s.autopilot_action, AutopilotAction::Mark);
    assert_eq!(s.autopilot_custom_prompt, "");

    // Fixture predates the voice scalars; decode-tolerant defaults apply.
    assert!(!s.voice_enabled);
    assert_eq!(s.voice_engine, "apple");
    assert_eq!(s.voice_push_to_talk_key, 54);
    assert!(s.voice_auto_insert);

    assert_eq!(s.saved_agents.len(), 1);
    let agent = &s.saved_agents[0];
    assert_eq!(agent.name, "Builder");
    assert_eq!(agent.agent_type, "claude");
    assert!(agent.is_companion);
    assert_eq!(agent.shell_command.as_deref(), Some("zsh -l"));
    assert_eq!(agent.persona_id,
               Some(Uuid::parse_str("a1000001-0000-0000-0000-000000000001").unwrap()));

    assert_eq!(s.saved_workspaces.len(), 1);
    let ws = &s.saved_workspaces[0];
    assert_eq!(ws.name, "Main");
    assert_eq!(ws.layout_mode, "splitVertical");
    assert_eq!(ws.split_ratio_secondary, Some(0.4));
    assert_eq!(ws.is_detached, Some(true));

    assert_eq!(s.personas.len(), 2);
    assert_eq!(s.personas[0].persona_type, PersonaType::System);
    assert_eq!(s.personas[0].state, PersonaState::Enabled);
    // Legacy record with no type/state falls back.
    assert_eq!(s.personas[1].persona_type, PersonaType::User);
    assert_eq!(s.personas[1].state, PersonaState::Enabled);

    assert_eq!(s.bench_agents.len(), 1);
    assert_eq!(s.bench_agents[0].agent_type, "codex");

    assert_eq!(s.recent_repos, vec!["app", "lib", "cli"]);
}

#[test]
fn reserializes_with_swift_keys() {
    let dir = legacy_store(FIXTURE);
    // Migrating writes each document, which is what the keys are read off.
    Settings::load_from_root(dir.path()).unwrap();

    let preferences = read_json(dir.path().join(PREFERENCES_FILE));
    let obj = preferences.as_object().unwrap();
    for key in ["appearanceMode",
                "restoreLayoutOnLaunch",
                "mcpServerPort",
                "sourceBaseFolderInitialized",
                "terminalFontName",
                "autopilotEnabled",
                "aiProvider",
                "aiApiKey",
                "autopilotAction",
                "autopilotCustomPrompt",
                "voiceEnabled",
                "voiceEngine",
                "voicePushToTalkKey",
                "voiceAutoInsert"]
    {
        assert!(obj.contains_key(key), "missing key {key}");
    }
    assert!(!obj.contains_key("paths"),
            "the store's paths must not serialize");
    // The collections have their own documents; a key here would be a second,
    // stale copy of each.
    for key in ["savedAgents",
                "savedWorkspaces",
                "personas",
                "benchAgents",
                "recentRepos"]
    {
        assert!(!obj.contains_key(key),
                "preferences document must not carry {key}");
    }

    let agents = read_json(dir.path().join(AGENTS_FILE));
    let agent = agents[0].as_object().unwrap();
    for key in ["agentType",
                "createdBy",
                "isCompanion",
                "shellCommand",
                "personaId"]
    {
        assert!(agent.contains_key(key), "saved agent missing key {key}");
    }

    let workspaces = read_json(dir.path().join(WORKSPACES_FILE));
    assert!(workspaces[0].as_object()
                         .unwrap()
                         .contains_key("layoutMode"));

    let personas = read_json(dir.path().join(PERSONAS_FILE));
    let persona = personas[0].as_object().unwrap();
    assert!(persona.contains_key("type"));
    assert!(persona.contains_key("state"));
}

/// The font-role migration is only correct if persisting records that it ran:
/// a second load that exchanged the values again would invert a user's fonts
/// on every launch.
#[test]
fn a_migrated_document_is_recorded_as_migrated() {
    let dir = legacy_store(r#"{"uiFontName":"Helvetica Neue","uiFontSize":13,
                               "titleFontName":"Palatino","titleFontSize":18}"#);

    let migrated = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(migrated.ui_font_name, "Palatino");
    assert_eq!(migrated.title_font_name, "Helvetica Neue");

    let written = read_json(dir.path().join(PREFERENCES_FILE));
    assert_eq!(written["settingsVersion"], serde_json::json!(1));

    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.ui_font_name, migrated.ui_font_name);
    assert_eq!(reloaded.ui_font_size, migrated.ui_font_size);
    assert_eq!(reloaded.title_font_name, migrated.title_font_name);
    assert_eq!(reloaded.title_font_size, migrated.title_font_size);
}

/// The divider's width is only durable if the write survives the round trip:
/// the drag handler sets the field and persists, and the next window to open
/// reads the document back.
#[test]
fn a_written_sidebar_width_survives_a_reload() {
    let dir = tempfile::tempdir().unwrap();

    let mut settings = Settings::with_store_root(dir.path());
    settings.sidebar_width = 180.0;
    settings.persist_preferences().unwrap();

    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.sidebar_width, 180.0);
}

/// The whole compatibility claim of #224 in one test: an existing settings
/// file keeps its meaning, and a rewritten one keeps its shape. If these
/// strings ever change, every user's stored appearance and autopilot choice
/// silently resets.
#[test]
fn the_vocabulary_wire_format_is_unchanged() {
    let dir = legacy_store(r#"{"appearanceMode":"dark","aiProvider":"google","autopilotAction":"continue"}"#);

    let loaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(loaded.appearance_mode, AppearanceMode::Dark);
    assert_eq!(loaded.ai_provider, AiProvider::Google);
    assert_eq!(loaded.autopilot_action, AutopilotAction::Continue);

    let written = read_json(dir.path().join(PREFERENCES_FILE));
    assert_eq!(written["appearanceMode"], "dark");
    assert_eq!(written["aiProvider"], "google");
    assert_eq!(written["autopilotAction"], "continue");
}

/// A corrupt value degrades to the default rather than failing the document.
#[test]
fn a_corrupt_vocabulary_value_does_not_take_the_document_down() {
    // `mcpServerPort` rather than a font field: an unmarked document also goes
    // through the font-role migration, which moves font values around and
    // would make this test about the wrong thing.
    let dir = legacy_store(r#"{"appearanceMode":"aut0","mcpServerPort":9111}"#);

    let loaded = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(loaded.appearance_mode, AppearanceMode::default());
    assert_eq!(loaded.mcp_server_port, 9111,
               "the rest of the document survived");
}

/// `agent-lifecycle` - "Legacy record without registry fields". The
/// fixture is a real Swift-era document: it predates the registry entirely,
/// so every agent and bench entry in it must load undescribed, untagged and
/// mid-priced rather than failing or being hidden.
#[test]
fn a_document_written_before_the_registry_loads_with_registry_defaults() {
    // Through the store, not `serde_json::from_str`: the collections live in
    // their own documents now, and a bare decode of the legacy blob would
    // leave them empty and assert nothing.
    let dir = legacy_store(FIXTURE);
    let settings = Settings::load_from_root(dir.path()).expect("fixture still loads");

    assert!(!settings.saved_agents.is_empty(),
            "fixture must exercise this");
    for agent in &settings.saved_agents {
        assert_eq!(agent.description, "");
        assert!(agent.capabilities.is_empty());
        assert_eq!(agent.cost_tier, CostTier::Medium);
    }
    for bench in &settings.bench_agents {
        assert_eq!(bench.description, "");
        assert!(bench.capabilities.is_empty());
        assert_eq!(bench.cost_tier, CostTier::Medium);
    }
}

/// Nothing about how an agent launches changes when it gains a tag, so the
/// three fields must survive a write/read cycle untouched.
#[test]
fn registry_metadata_survives_a_settings_round_trip() {
    let dir = legacy_store(FIXTURE);
    let mut settings = Settings::load_from_root(dir.path()).expect("fixture loads");
    settings.saved_agents[0].description = "Runs the test suite".to_string();
    settings.saved_agents[0].capabilities = [" Testing ", "rust"].iter().collect();
    settings.saved_agents[0].cost_tier = CostTier::Low;

    // The round trip is now write-then-read through the store: the agents
    // document is where these three fields have to survive.
    settings.persist().unwrap();
    let back = Settings::load_from_root(dir.path()).unwrap();

    let agent = &back.saved_agents[0];
    assert_eq!(agent.description, "Runs the test suite");
    assert!(agent.capabilities.contains("testing"),
            "normalized on the way in");
    assert!(agent.capabilities.contains("rust"));
    assert_eq!(agent.cost_tier, CostTier::Low);
}
