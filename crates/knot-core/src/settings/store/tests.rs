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
