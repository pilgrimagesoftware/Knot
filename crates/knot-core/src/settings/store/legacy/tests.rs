//! Unit tests for [`super`]: the one-time migration off `settings.json`.

use std::fs;

use tempfile::TempDir;
use tempfile::tempdir;

use super::*;
use crate::consts::{
    AGENTS_FILE, BENCH_FILE, LEGACY_SETTINGS_FILE, PERSONAS_FILE, PREFERENCES_FILE,
    RECENT_REPOS_FILE, SETTINGS_VERSION_CURRENT, WORKSPACES_FILE,
};
use crate::settings::{PersonaState, PersonaType};

/// A legacy document carrying a value for every scalar kind and every
/// collection, so a migration that drops one is visible.
const FULL_LEGACY_DOCUMENT: &str = r#"{
    "settingsVersion": 1,
    "appearanceMode": "dark",
    "restoreLayoutOnLaunch": false,
    "restoreConversationOnLaunch": true,
    "mcpServerPort": 9300,
    "sourceBaseFolder": "/src",
    "sourceBaseFolderInitialized": true,
    "mermaidScale": 1.5,
    "agentCommands": {"claude": "claude --resume"},
    "terminalFontName": "Fira Code",
    "sidebarWidth": 320,
    "voicePushToTalkKey": 55,
    "agentPanelCompactToolCalls": true,
    "savedWorkspaces": [],
    "personas": [
        {"id": "3f2504e0-4f89-41d3-9a0c-0305e82c3301", "name": "Rookie",
         "instructions": "be helpful", "type": "user", "state": "enabled"}
    ],
    "benchAgents": [],
    "recentRepos": ["alpha", "beta"]
}"#;

fn write_legacy(dir: &TempDir, document: &str) {
    fs::write(dir.path().join(LEGACY_SETTINGS_FILE), document).unwrap();
}

fn migrated_path(dir: &TempDir) -> std::path::PathBuf {
    dir.path()
       .join(LEGACY_SETTINGS_FILE)
       .with_extension(LEGACY_MIGRATED_EXTENSION)
}

#[test]
fn an_upgraded_installation_keeps_every_value_and_retires_the_legacy_document() {
    let dir = tempdir().unwrap();
    write_legacy(&dir, FULL_LEGACY_DOCUMENT);

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.mcp_server_port, 9300);
    assert!(!settings.restore_layout_on_launch);
    assert!(settings.restore_conversation_on_launch);
    assert_eq!(settings.source_base_folder, "/src");
    assert!(settings.source_folder_detected);
    assert_eq!(settings.mermaid_scale, 1.5);
    assert_eq!(settings.agent_commands.get("claude").map(String::as_str),
               Some("claude --resume"));
    assert_eq!(settings.terminal_font_name, "Fira Code");
    assert_eq!(settings.sidebar_width, 320.0);
    assert_eq!(settings.voice_push_to_talk_key, 55);
    assert!(settings.agent_panel_compact_tool_calls);
    assert_eq!(settings.personas.len(), 1);
    assert_eq!(settings.personas[0].name, "Rookie");
    assert_eq!(settings.personas[0].persona_type, PersonaType::User);
    assert_eq!(settings.personas[0].state, PersonaState::Enabled);
    assert_eq!(settings.recent_repos, vec!["alpha", "beta"]);

    for file in [PREFERENCES_FILE,
                 AGENTS_FILE,
                 WORKSPACES_FILE,
                 PERSONAS_FILE,
                 BENCH_FILE,
                 RECENT_REPOS_FILE]
    {
        assert!(dir.path().join(file).exists(), "{file} was not written");
    }
    assert!(!dir.path().join(LEGACY_SETTINGS_FILE).exists());
    assert_eq!(fs::read_to_string(migrated_path(&dir)).unwrap(),
               FULL_LEGACY_DOCUMENT);
}

#[test]
fn the_migrated_store_reloads_to_the_same_values() {
    let dir = tempdir().unwrap();
    write_legacy(&dir, FULL_LEGACY_DOCUMENT);

    let migrated = Settings::load_from_root(dir.path()).unwrap();
    let reloaded = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(migrated, reloaded);
}

#[test]
fn migration_does_not_run_twice() {
    let dir = tempdir().unwrap();
    write_legacy(&dir, FULL_LEGACY_DOCUMENT);
    Settings::load_from_root(dir.path()).unwrap();

    // A change made after the migration must survive the second load, which
    // is only true if nothing re-reads the legacy document.
    let mut settings = Settings::load_from_root(dir.path()).unwrap();
    settings.add_recent_repo("gamma").unwrap();
    let reloaded = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(reloaded.recent_repos, vec!["gamma", "alpha", "beta"]);
    assert!(!dir.path().join(LEGACY_SETTINGS_FILE).exists());
}

#[test]
fn a_written_document_wins_over_the_legacy_one() {
    let dir = tempdir().unwrap();
    write_legacy(&dir, FULL_LEGACY_DOCUMENT);
    fs::write(dir.path().join(PERSONAS_FILE),
              r#"[{"id":"3f2504e0-4f89-41d3-9a0c-0305e82c3302","name":"Veteran",
                   "instructions":"be terse","type":"user","state":"enabled"}]"#).unwrap();

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.personas.len(), 1);
    assert_eq!(settings.personas[0].name, "Veteran");
    // Everything the written document did not cover still comes from the
    // legacy one.
    assert_eq!(settings.recent_repos, vec!["alpha", "beta"]);
}

/// The state an interrupted migration leaves behind: some new documents
/// written, the legacy one not yet renamed. Finishing from there must land on
/// exactly what a clean run would have produced.
#[test]
fn an_interrupted_migration_finishes_on_the_next_load() {
    let clean = tempdir().unwrap();
    write_legacy(&clean, FULL_LEGACY_DOCUMENT);
    let expected = Settings::load_from_root(clean.path()).unwrap();

    let interrupted = tempdir().unwrap();
    write_legacy(&interrupted, FULL_LEGACY_DOCUMENT);
    // What the crashed run had managed to write: the personas document,
    // holding exactly what the legacy document says.
    fs::copy(clean.path().join(PERSONAS_FILE),
             interrupted.path().join(PERSONAS_FILE)).unwrap();

    let finished = Settings::load_from_root(interrupted.path()).unwrap();

    assert_eq!(finished.personas, expected.personas);
    assert_eq!(finished.recent_repos, expected.recent_repos);
    assert_eq!(finished.mcp_server_port, expected.mcp_server_port);
    assert!(!interrupted.path().join(LEGACY_SETTINGS_FILE).exists());
    assert!(migrated_path(&interrupted).exists());
}

#[test]
fn an_unreadable_legacy_document_is_left_in_place() {
    let dir = tempdir().unwrap();
    write_legacy(&dir, "{ not json");

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings, Settings::with_store_root(dir.path()));
    assert!(dir.path().join(LEGACY_SETTINGS_FILE).exists());
    assert!(!migrated_path(&dir).exists());
    assert!(!dir.path().join(PREFERENCES_FILE).exists());
}

#[test]
fn a_non_object_legacy_document_is_left_in_place() {
    let dir = tempdir().unwrap();
    write_legacy(&dir, "[1, 2, 3]");

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings, Settings::with_store_root(dir.path()));
    assert!(dir.path().join(LEGACY_SETTINGS_FILE).exists());
}

#[test]
fn a_collection_record_that_cannot_be_decoded_is_dropped_not_fatal() {
    let dir = tempdir().unwrap();
    write_legacy(&dir,
                 r#"{"settingsVersion":1,"mcpServerPort":9000,
                     "personas":[{"id":"not-a-uuid"},
                                 {"id":"3f2504e0-4f89-41d3-9a0c-0305e82c3301",
                                  "name":"Rookie","instructions":""}],
                     "recentRepos":"broken"}"#);

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.mcp_server_port, 9000);
    assert_eq!(settings.personas.len(), 1);
    assert_eq!(settings.personas[0].name, "Rookie");
    // A legacy persona with no type or state reads as an enabled user
    // persona, the same as it did before the split.
    assert_eq!(settings.personas[0].persona_type, PersonaType::User);
    assert_eq!(settings.personas[0].state, PersonaState::Enabled);
    assert!(settings.recent_repos.is_empty());
}

/// The font-role swap is gated on a marker carried in the preferences
/// document, so the migration has to run it on the way past - otherwise an
/// installation that upgrades through this change has its fonts left
/// exchanged for good.
#[test]
fn font_roles_are_migrated_out_of_a_legacy_document() {
    let dir = tempdir().unwrap();
    write_legacy(&dir,
                 r#"{"uiFontName":"Helvetica Neue","titleFontName":"Palatino"}"#);

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.ui_font_name, "Palatino");
    assert_eq!(settings.title_font_name, "Helvetica Neue");
    assert_eq!(settings.settings_version, SETTINGS_VERSION_CURRENT);

    let written = fs::read_to_string(dir.path().join(PREFERENCES_FILE)).unwrap();
    let written: serde_json::Value = serde_json::from_str(&written).unwrap();
    assert_eq!(written["uiFontName"], "Palatino");
    assert_eq!(written["titleFontName"], "Helvetica Neue");
    assert_eq!(written["settingsVersion"], SETTINGS_VERSION_CURRENT);
}

#[test]
fn nothing_happens_without_a_legacy_document() {
    let dir = tempdir().unwrap();

    assert!(migrate(&StorePaths::rooted(dir.path())).unwrap().is_none());
    assert!(!dir.path().join(PREFERENCES_FILE).exists());
}
