//! Unit tests for [`super`].

use tempfile::tempdir;

use super::*;

fn agent_id() -> Uuid {
    Uuid::new_v4()
}

#[test]
fn default_scalars() {
    let s = Settings::default();
    assert_eq!(s.mcp_server_port, 8767);
    assert_eq!(s.terminal_font_name, "JetBrains Mono");
    assert!(s.restore_layout_on_launch);
    assert!(!s.restore_conversation_on_launch);
    assert!(s.mcp_server_enabled);
}

#[test]
fn legacy_settings_blob_defaults_restore_conversation_off() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"restoreLayoutOnLaunch":true}"#).unwrap();
    let s = Settings::load_from(&path).unwrap();
    assert!(s.restore_layout_on_launch);
    assert!(!s.restore_conversation_on_launch);
}

#[test]
fn persisted_sf_mono_upgrades_to_the_new_terminal_font_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"terminalFontName":"SF Mono"}"#).unwrap();
    let s = Settings::load_from(&path).unwrap();
    assert_eq!(s.terminal_font_name, "JetBrains Mono");
}

#[test]
fn persisted_custom_terminal_font_is_not_overridden() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"terminalFontName":"Fira Code"}"#).unwrap();
    let s = Settings::load_from(&path).unwrap();
    assert_eq!(s.terminal_font_name, "Fira Code");
}

/// Writes `document` to a fresh store and loads it back, so each font-role
/// migration case reads as the document it is about.
fn load_document(document: &str) -> (tempfile::TempDir, Settings) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, document).unwrap();
    let settings = Settings::load_from(&path).unwrap();
    (dir, settings)
}

#[test]
fn a_fresh_store_is_already_at_the_current_settings_version() {
    assert_eq!(Settings::default().settings_version,
               SETTINGS_VERSION_CURRENT);
}

#[test]
fn a_document_without_the_marker_reads_as_pre_migration() {
    // Not the container-level default, which is `SETTINGS_VERSION_CURRENT` -
    // the field's own serde default has to win on the deserialize path, or an
    // unmigrated document would declare itself migrated.
    assert_eq!(de_legacy_settings_version(), 0);

    let mut document: Value = serde_json::from_str("{}").unwrap();
    let object = document.as_object_mut().unwrap();
    assert!(!object.contains_key("settingsVersion"));

    let settings: Settings = serde_json::from_value(document).unwrap();
    assert_eq!(settings.settings_version, 0);
}

#[test]
fn the_font_defaults_name_what_they_draw() {
    let s = Settings::default();
    assert_eq!(s.ui_font_name, "Adamina");
    assert_eq!(s.ui_font_size, 16.0);
    assert_eq!(s.title_font_name, "Manrope");
    assert_eq!(s.title_font_size, 14.0);
}

#[test]
fn both_customized_fonts_are_exchanged() {
    let (_dir, s) = load_document(r#"{"uiFontName":"Helvetica Neue","uiFontSize":13,
                                      "titleFontName":"Palatino","titleFontSize":18}"#);
    assert_eq!(s.ui_font_name, "Palatino");
    assert_eq!(s.ui_font_size, 18.0);
    assert_eq!(s.title_font_name, "Helvetica Neue");
    assert_eq!(s.title_font_size, 13.0);
    assert_eq!(s.settings_version, SETTINGS_VERSION_CURRENT);
}

#[test]
fn one_customized_font_moves_and_the_other_takes_the_new_default() {
    let (_dir, s) = load_document(r#"{"titleFontName":"Palatino"}"#);
    assert_eq!(s.ui_font_name, "Palatino");
    // Not "Palatino" - an absent key stays absent through the exchange rather
    // than inheriting the other's value.
    assert_eq!(s.title_font_name, "Manrope");
}

#[test]
fn a_document_that_customized_neither_font_keeps_the_new_defaults() {
    let (_dir, s) = load_document(r#"{"appearanceMode":"dark"}"#);
    assert_eq!(s.ui_font_name, "Adamina");
    assert_eq!(s.ui_font_size, 16.0);
    assert_eq!(s.title_font_name, "Manrope");
    assert_eq!(s.title_font_size, 14.0);
}

#[test]
fn the_migration_does_not_run_twice() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,
                                      "uiFontName":"Adamina","titleFontName":"Manrope"}"#);
    assert_eq!(s.ui_font_name, "Adamina");
    assert_eq!(s.title_font_name, "Manrope");
}

#[test]
fn the_migration_leaves_the_terminal_font_alone() {
    let (_dir, s) = load_document(r#"{"terminalFontName":"Fira Code","terminalFontSize":11,
                                      "uiFontName":"Helvetica Neue","titleFontName":"Palatino"}"#);
    assert_eq!(s.terminal_font_name, "Fira Code");
    assert_eq!(s.terminal_font_size, 11.0);
}

#[test]
fn a_fresh_store_opens_the_sidebar_at_the_default_width() {
    assert_eq!(Settings::default().sidebar_width, SIDEBAR_WIDTH_DEFAULT);
}

#[test]
fn a_document_without_the_sidebar_width_takes_the_default() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"mcpServerPort":9000}"#);
    assert_eq!(s.mcp_server_port, 9000);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_DEFAULT);
}

#[test]
fn an_in_range_sidebar_width_is_honored() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":320}"#);
    assert_eq!(s.sidebar_width, 320.0);
}

#[test]
fn a_sidebar_width_below_the_minimum_clamps_up() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":40}"#);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_MIN);
}

#[test]
fn a_sidebar_width_above_the_maximum_clamps_down() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":5000}"#);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_MAX);
}

#[test]
fn a_non_numeric_sidebar_width_leaves_the_default() {
    let (_dir, s) = load_document(r#"{"settingsVersion":1,"sidebarWidth":"wide"}"#);
    assert_eq!(s.sidebar_width, SIDEBAR_WIDTH_DEFAULT);
}

#[test]
fn missing_file_yields_defaults() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let s = Settings::load_from(&path).unwrap();
    assert_eq!(s, Settings::with_store_path(&path));
}

#[test]
fn corrupt_file_yields_defaults() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    let s = Settings::load_from(&path).unwrap();
    assert_eq!(s.mcp_server_port, 8767);
    assert!(s.saved_agents.is_empty());
}

#[test]
fn scalar_persists_across_reload() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut s = Settings::with_store_path(&path);
    s.mcp_server_port = 9000;
    s.persist().unwrap();
    let reloaded = Settings::load_from(&path).unwrap();
    assert_eq!(reloaded.mcp_server_port, 9000);
}

#[test]
fn one_broken_collection_does_not_sink_the_rest() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path,
              r#"{"mcpServerPort":9100,"savedAgents":"broken","recentRepos":["a","b"]}"#).unwrap();
    let s = Settings::load_from(&path).unwrap();
    assert_eq!(s.mcp_server_port, 9100);
    assert!(s.saved_agents.is_empty());
    assert_eq!(s.recent_repos, vec!["a", "b"]);
}

#[test]
fn detect_picks_first_existing() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("src");
    let present = dir.path().join("source");
    fs::create_dir(&present).unwrap();
    let candidates: Vec<&Path> = vec![missing.as_path(), present.as_path()];
    assert_eq!(detect_source_base_folder(&candidates), Some(present));
}

#[test]
fn init_source_folder_runs_once() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut s = Settings::with_store_path(&path);
    s.init_source_folder().unwrap();
    assert!(s.source_folder_detected);
    s.source_base_folder = "/explicit".to_string();
    s.init_source_folder().unwrap();
    assert_eq!(s.source_base_folder, "/explicit");
}

#[test]
fn recent_repos_moves_to_front_and_caps() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    for name in ["c", "b", "a"] {
        s.add_recent_repo(name).unwrap();
    }
    assert_eq!(s.recent_repos, vec!["a", "b", "c"]);
    s.add_recent_repo("b").unwrap();
    assert_eq!(s.recent_repos, vec!["b", "a", "c"]);
    for name in ["d", "e", "f"] {
        s.add_recent_repo(name).unwrap();
    }
    assert_eq!(s.recent_repos.len(), RECENT_REPOS_MAX);
    assert_eq!(s.recent_repos[0], "f");
}

#[test]
fn bench_replaces_same_folder() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    s.add_bench_agent(BenchAgent::new(agent_id(), "old", None, "/repo"))
     .unwrap();
    s.add_bench_agent(BenchAgent::new(agent_id(), "new", None, "/repo"))
     .unwrap();
    assert_eq!(s.bench_agents.len(), 1);
    assert_eq!(s.bench_agents[0].name, "new");
}

#[test]
fn active_personas_excludes_deleted_and_sorts_ci() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    s.personas = vec![Persona { id:           agent_id(),
                                name:         "beta".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::User,
                                state:        PersonaState::Enabled, },
                      Persona { id:           agent_id(),
                                name:         "Alpha".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::User,
                                state:        PersonaState::Enabled, },
                      Persona { id:           agent_id(),
                                name:         "gone".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::System,
                                state:        PersonaState::Deleted, },];
    let names: Vec<&str> = s.active_personas()
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect();
    assert_eq!(names, vec!["Alpha", "beta"]);
}

#[test]
fn default_personas_install_once() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    s.install_default_personas().unwrap();
    assert_eq!(s.personas.len(), 6);
    s.install_default_personas().unwrap();
    assert_eq!(s.personas.len(), 6);
}

#[test]
fn add_update_and_lookup_persona() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    let id = s.add_persona("Rookie", "be helpful").unwrap().id;
    assert_eq!(s.personas.len(), 1);
    assert_eq!(s.persona(id).unwrap().name, "Rookie");

    s.update_persona(id, "Veteran", "be terse").unwrap();
    let persona = s.persona(id).unwrap();
    assert_eq!(persona.name, "Veteran");
    assert_eq!(persona.instructions, "be terse");

    s.update_persona(agent_id(), "Nobody", "").unwrap();
    assert_eq!(s.personas.len(), 1);
}

#[test]
fn persona_lookup_excludes_deleted() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    s.personas = vec![Persona { id:           agent_id(),
                                name:         "gone".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::System,
                                state:        PersonaState::Deleted, }];
    assert!(s.persona(s.personas[0].id).is_none());
}

#[test]
fn remove_persona_soft_deletes_system_and_hard_deletes_user() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    let system_id = agent_id();
    let user_id = agent_id();
    s.personas = vec![Persona { id:           system_id,
                                name:         "System".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::System,
                                state:        PersonaState::Enabled, },
                      Persona { id:           user_id,
                                name:         "User".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::User,
                                state:        PersonaState::Enabled, },];

    s.remove_persona(system_id).unwrap();
    assert_eq!(s.personas.len(), 2);
    assert_eq!(s.personas.iter().find(|p| p.id == system_id).unwrap().state,
               PersonaState::Deleted);

    s.remove_persona(user_id).unwrap();
    assert_eq!(s.personas.len(), 1);
    assert!(s.personas.iter().all(|p| p.id != user_id));

    s.remove_persona(agent_id()).unwrap();
    assert_eq!(s.personas.len(), 1);
}

#[test]
fn restore_default_personas_reverts_edits_and_adds_missing() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    let (id, name, instructions) = DEFAULT_PERSONAS[0];
    let id = Uuid::parse_str(id).unwrap();
    s.personas = vec![Persona { id,
                                name: "Renamed".to_string(),
                                instructions: "different".to_string(),
                                persona_type: PersonaType::System,
                                state: PersonaState::Disabled },
                      Persona { id:           agent_id(),
                                name:         "Mine".to_string(),
                                instructions: "keep me".to_string(),
                                persona_type: PersonaType::User,
                                state:        PersonaState::Enabled, },];

    s.restore_default_personas().unwrap();

    assert_eq!(s.personas.len(), 7);
    let restored = s.personas.iter().find(|p| p.id == id).unwrap();
    assert_eq!(restored.name, name);
    assert_eq!(restored.instructions, instructions);
    assert_eq!(restored.state, PersonaState::Enabled);
    assert!(s.personas.iter().any(|p| p.name == "Mine"));
}

#[test]
fn deleted_default_persona_not_reinstalled() {
    let dir = tempdir().unwrap();
    let mut s = Settings::with_store_path(dir.path().join("settings.json"));
    let (id, _, _) = DEFAULT_PERSONAS[0];
    s.personas = vec![Persona { id:           Uuid::parse_str(id).unwrap(),
                                name:         "custom".to_string(),
                                instructions: String::new(),
                                persona_type: PersonaType::System,
                                state:        PersonaState::Deleted, }];
    s.install_default_personas().unwrap();
    assert_eq!(s.personas.len(), 6);
    assert_eq!(s.personas
                .iter()
                .filter(|p| p.state == PersonaState::Deleted)
                .count(),
               1);
}

#[test]
fn persist_leaves_no_temporary_file_behind() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(SETTINGS_FILE);
    let settings = Settings::with_store_path(&path);

    settings.persist().unwrap();

    assert!(path.exists());
    assert!(!path.with_extension(SETTINGS_TEMP_EXTENSION).exists());
}

#[test]
fn persist_replaces_a_stale_temporary_file_rather_than_reusing_it() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(SETTINGS_FILE);
    let temporary = path.with_extension(SETTINGS_TEMP_EXTENSION);
    // What an interrupted write would leave: a partial document that
    // must not become the next persisted one.
    fs::write(&temporary, "{ not json").unwrap();

    let mut settings = Settings::with_store_path(&path);
    settings.ui_font_size = 17.0;
    settings.persist().unwrap();

    assert!(!temporary.exists());
    let reloaded = Settings::load_from(&path).unwrap();
    assert_eq!(reloaded.ui_font_size, 17.0);
}

#[test]
fn legacy_settings_blob_defaults_compact_tool_calls_off() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"restoreLayoutOnLaunch":true}"#).unwrap();
    let s = Settings::load_from(&path).unwrap();
    assert!(!s.agent_panel_compact_tool_calls);
}

#[test]
fn compact_tool_calls_round_trips_through_the_store() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut s = Settings::load_from(&path).unwrap();
    assert!(!s.agent_panel_compact_tool_calls);
    s.agent_panel_compact_tool_calls = true;
    s.persist().unwrap();

    let reloaded = Settings::load_from(&path).unwrap();
    assert!(reloaded.agent_panel_compact_tool_calls);
}
