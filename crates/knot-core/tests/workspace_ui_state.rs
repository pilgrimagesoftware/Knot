//! Guards the split of the combined `workspaces.json` into a saved-workspaces
//! document and a per-workspace UI-state document.
//!
//! The fixture is a capture from a real installation - six workspaces, three
//! of them moved - with only its names and identifiers replaced. Captured
//! rather than hand-written on purpose: serde ignores unknown fields, so a
//! combined document decodes cleanly into the narrowed `Workspace` while
//! dropping every arrangement it held, with no error and nothing to notice
//! until windows open centred. A fixture written to match the code cannot
//! catch that; one written by the code that produced the old shape can.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.

use std::collections::BTreeMap;

use knot_core::consts::{WORKSPACE_UI_STATE_FILE, WORKSPACES_FILE};
use knot_core::{Settings, WorkspaceUiState};
use uuid::Uuid;

const COMBINED: &str = include_str!("fixtures/workspaces_combined.json");

/// The eight keys that move out of the workspace record.
const UI_KEYS: [&str; 8] = ["layoutMode",
                            "activeAgentIds",
                            "focusedPaneIndex",
                            "splitRatio",
                            "splitRatioSecondary",
                            "showDashboard",
                            "isDetached",
                            "windowBounds"];

/// The four that stay on it.
const CONFIGURED_KEYS: [&str; 4] = ["id", "name", "colorHex", "agentIds"];

fn combined_store(document: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(WORKSPACES_FILE), document).unwrap();
    dir
}

fn read_json(path: std::path::PathBuf) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn id(name: &str) -> Uuid {
    let fixture: serde_json::Value = serde_json::from_str(COMBINED).unwrap();
    let record = fixture.as_array()
                        .unwrap()
                        .iter()
                        .find(|r| r["name"] == name)
                        .unwrap_or_else(|| panic!("fixture has no workspace named {name}"));
    Uuid::parse_str(record["id"].as_str().unwrap()).unwrap()
}

/// What the fixture says each workspace's arrangement is, read straight out of
/// the JSON rather than restated here: a copy of the values would drift from
/// the fixture and start asserting itself.
fn expected_ui() -> BTreeMap<Uuid, serde_json::Value> {
    let fixture: serde_json::Value = serde_json::from_str(COMBINED).unwrap();
    fixture.as_array()
           .unwrap()
           .iter()
           .map(|record| {
               let key = Uuid::parse_str(record["id"].as_str().unwrap()).unwrap();
               let ui: serde_json::Map<String, serde_json::Value> =
                   UI_KEYS.iter()
                          .map(|k| ((*k).to_string(), record[*k].clone()))
                          .collect();
               (key, serde_json::Value::Object(ui))
           })
           .collect()
}

/// The test the migration exists for: every configured value lands in the
/// workspaces document and every arrangement value in the UI-state document,
/// field by field, with nothing lost in between.
#[test]
fn a_combined_document_splits_without_losing_a_value() {
    let dir = combined_store(COMBINED);

    let settings = Settings::load_from_root(dir.path()).unwrap();

    let fixture: serde_json::Value = serde_json::from_str(COMBINED).unwrap();
    let records = fixture.as_array().unwrap();
    assert_eq!(settings.saved_workspaces.len(),
               records.len(),
               "every workspace survived the split");

    for record in records {
        let key = Uuid::parse_str(record["id"].as_str().unwrap()).unwrap();

        // Configuration: on the workspace, value for value.
        let workspace = settings.saved_workspaces
                                .iter()
                                .find(|w| w.id == key)
                                .unwrap_or_else(|| panic!("workspace {key} was dropped"));
        assert_eq!(workspace.name, record["name"].as_str().unwrap());
        assert_eq!(workspace.color_hex, record["colorHex"].as_str().unwrap());
        let agent_ids: Vec<Uuid> =
            record["agentIds"].as_array()
                              .unwrap()
                              .iter()
                              .map(|v| Uuid::parse_str(v.as_str().unwrap()).unwrap())
                              .collect();
        assert_eq!(workspace.agent_ids, agent_ids);

        // Arrangement: in the UI-state map, value for value.
        let ui = settings.workspace_ui
                         .get(&key)
                         .unwrap_or_else(|| panic!("no UI state for workspace {key}"));
        assert_eq!(ui.layout_mode, record["layoutMode"].as_str().unwrap());
        let active: Vec<Uuid> =
            record["activeAgentIds"].as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|v| Uuid::parse_str(v.as_str().unwrap()).unwrap())
                                    .collect();
        assert_eq!(ui.active_agent_ids, active);
        assert_eq!(i64::from(ui.focused_pane_index),
                   record["focusedPaneIndex"].as_i64().unwrap());
        assert_eq!(ui.split_ratio, record["splitRatio"].as_f64().unwrap());
        assert_eq!(ui.split_ratio_secondary,
                   record["splitRatioSecondary"].as_f64());
        assert_eq!(ui.show_dashboard, record["showDashboard"].as_bool());
        assert_eq!(ui.is_detached, record["isDetached"].as_bool());
        match record["windowBounds"].as_object() {
            Some(bounds) => {
                let saved = ui.window_bounds
                              .unwrap_or_else(|| panic!("workspace {key} lost its bounds"));
                assert_eq!(f64::from(saved.x), bounds["x"].as_f64().unwrap());
                assert_eq!(f64::from(saved.y), bounds["y"].as_f64().unwrap());
                assert_eq!(f64::from(saved.width), bounds["width"].as_f64().unwrap());
                assert_eq!(f64::from(saved.height), bounds["height"].as_f64().unwrap());
            }
            None => assert!(ui.window_bounds.is_none(),
                            "workspace {key} gained bounds it never had"),
        }
    }
}

/// The split is only done if it is also written: a migration that loads
/// correctly but leaves the combined document on disk runs again every launch
/// and never narrows the record.
#[test]
fn the_split_is_written_to_both_documents() {
    let dir = combined_store(COMBINED);

    Settings::load_from_root(dir.path()).unwrap();

    let workspaces = read_json(dir.path().join(WORKSPACES_FILE));
    for record in workspaces.as_array().unwrap() {
        let object = record.as_object().unwrap();
        for key in CONFIGURED_KEYS {
            assert!(object.contains_key(key),
                    "workspace record lost configured key {key}");
        }
        for key in UI_KEYS {
            assert!(!object.contains_key(key),
                    "workspace record still carries UI key {key}");
        }
    }

    let ui_document = read_json(dir.path().join(WORKSPACE_UI_STATE_FILE));
    let entries = ui_document.as_object()
                             .expect("the UI-state document is a map keyed by workspace id");
    assert_eq!(entries.len(), expected_ui().len());
    for (key, expected) in expected_ui() {
        let written = entries.get(&key.to_string())
                             .unwrap_or_else(|| panic!("UI-state document has no entry for {key}"));
        for ui_key in UI_KEYS {
            assert_eq!(&written[ui_key], &expected[ui_key],
                       "workspace {key} key {ui_key} changed in the move");
        }
    }
}

/// Arrangement is only preserved if it survives the round trip, not merely the
/// load: the launch after the upgrade reads the written documents, not the
/// combined one.
#[test]
fn arrangement_survives_the_launch_after_the_upgrade() {
    let dir = combined_store(COMBINED);

    let migrated = Settings::load_from_root(dir.path()).unwrap();
    let reloaded = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(reloaded.saved_workspaces, migrated.saved_workspaces);
    assert_eq!(reloaded.workspace_ui, migrated.workspace_ui);

    let moved = id("Alpha");
    let bounds = reloaded.workspace_ui[&moved].window_bounds
                                              .expect("Alpha's window was moved");
    assert_eq!((bounds.x, bounds.y, bounds.width, bounds.height),
               (10.0, 42.0, 1708.0, 1065.0));
    assert_eq!(reloaded.workspace_ui[&id("Delta")].split_ratio,
               0.7087120378521127);
    assert_eq!(reloaded.workspace_ui[&id("Foxtrot")].show_dashboard,
               Some(true));
}

/// A record whose arrangement keys were never written - the shape a workspace
/// created before a UI field existed has - migrates with that field defaulted
/// rather than failing the record.
#[test]
fn a_combined_record_missing_ui_fields_migrates_with_defaults() {
    let dir = combined_store(r##"[{"id": "0a94f36c-8760-4bd2-b210-c2758f56cf5d",
                                   "name": "Sparse", "colorHex": "#123456",
                                   "agentIds": [], "windowBounds": null,
                                   "layoutMode": "single"}]"##);

    let settings = Settings::load_from_root(dir.path()).unwrap();

    let key = Uuid::parse_str("0a94f36c-8760-4bd2-b210-c2758f56cf5d").unwrap();
    assert_eq!(settings.saved_workspaces[0].name, "Sparse");
    let ui = &settings.workspace_ui[&key];
    assert_eq!(ui,
               &WorkspaceUiState { layout_mode: "single".to_string(),
                                   ..WorkspaceUiState::default() });
}

/// Already split, so there is nothing to migrate: the load must not rewrite
/// either document. A migration that runs every launch would also rewrite the
/// roster every launch, which is the coupling the change removes.
#[test]
fn an_already_split_installation_is_not_rewritten_by_a_load() {
    let dir = combined_store(COMBINED);
    Settings::load_from_root(dir.path()).unwrap();

    let workspaces_path = dir.path().join(WORKSPACES_FILE);
    let ui_path = dir.path().join(WORKSPACE_UI_STATE_FILE);
    let workspaces_before = std::fs::read(&workspaces_path).unwrap();
    let ui_before = std::fs::read(&ui_path).unwrap();

    Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(std::fs::read(&workspaces_path).unwrap(),
               workspaces_before,
               "a second load rewrote the workspaces document");
    assert_eq!(std::fs::read(&ui_path).unwrap(),
               ui_before,
               "a second load rewrote the UI-state document");
}
