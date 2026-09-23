//! Unit tests for [`super`]: the one-time split of a combined
//! `workspaces.json`.
//!
//! The migration's whole-document behaviour - every value preserved, field by
//! field, against a fixture captured from a real installation - is asserted in
//! `tests/workspace_ui_state.rs`. What is here is the awkward states that
//! fixture cannot be in at once: a crash part-way, a missing document, an
//! entry that will not decode, and an entry outliving its workspace.

use std::fs;

use tempfile::{TempDir, tempdir};

use super::*;
use crate::consts::{LEGACY_SETTINGS_FILE, WORKSPACE_UI_STATE_FILE, WORKSPACES_FILE};
use crate::settings::Settings;

const ALPHA: &str = "0a94f36c-8760-4bd2-b210-c2758f56cf5d";
const BRAVO: &str = "c74d260f-41f5-437c-8d5d-eb8d66f2c438";

fn alpha() -> Uuid {
    Uuid::parse_str(ALPHA).unwrap()
}

fn bravo() -> Uuid {
    Uuid::parse_str(BRAVO).unwrap()
}

/// Two combined records: one moved and split, one never arranged.
fn combined_document() -> String {
    format!(
            r##"[
        {{"id": "{ALPHA}", "name": "Alpha", "colorHex": "#101010",
          "agentIds": [], "layoutMode": "splitVertical", "activeAgentIds": [],
          "focusedPaneIndex": 2, "splitRatio": 0.25,
          "splitRatioSecondary": 0.4, "showDashboard": true,
          "isDetached": true,
          "windowBounds": {{"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}}}},
        {{"id": "{BRAVO}", "name": "Bravo", "colorHex": "#202020",
          "agentIds": [], "layoutMode": "single", "activeAgentIds": [],
          "focusedPaneIndex": 0, "splitRatio": 0.5,
          "splitRatioSecondary": null, "showDashboard": null,
          "isDetached": null, "windowBounds": null}}
    ]"##
    )
}

fn write_workspaces(dir: &TempDir, document: &str) {
    fs::write(dir.path().join(WORKSPACES_FILE), document).unwrap();
}

fn write_ui_state(dir: &TempDir, document: &str) {
    fs::write(dir.path().join(WORKSPACE_UI_STATE_FILE), document).unwrap();
}

#[test]
fn a_combined_document_is_detected_and_split() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir, &combined_document());

    let split = read(&StorePaths::rooted(dir.path()));

    assert!(split.needs_write,
            "a combined document must be written back");
    assert_eq!(split.workspaces.len(), 2);
    let ui = &split.ui_state[&alpha()];
    assert_eq!(ui.layout_mode, "splitVertical");
    assert_eq!(ui.focused_pane_index, 2);
    assert_eq!(ui.split_ratio, 0.25);
    assert_eq!(ui.split_ratio_secondary, Some(0.4));
    assert_eq!(ui.show_dashboard, Some(true));
    assert_eq!(ui.is_detached, Some(true));
    assert_eq!(ui.window_bounds.unwrap().width, 3.0);
}

/// The crash-part-way case: the UI-state document was written and the process
/// died before the workspaces document was rewritten, so the next launch still
/// sees a combined document. What the user has done since is in the written
/// entries, and must not be overwritten by the stale combined fields.
#[test]
fn an_entry_already_written_wins_over_the_combined_record() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir, &combined_document());
    write_ui_state(&dir,
                   &format!(r##"{{"{ALPHA}": {{"layoutMode": "single",
                                              "focusedPaneIndex": 0,
                                              "splitRatio": 0.9}}}}"##));

    let split = read(&StorePaths::rooted(dir.path()));

    let alpha_ui = &split.ui_state[&alpha()];
    assert_eq!(alpha_ui.layout_mode, "single", "the written entry won");
    assert_eq!(alpha_ui.split_ratio, 0.9);
    assert!(split.needs_write, "the split still has to finish");
    // The record the crash never reached is taken from the combined document.
    assert!(split.ui_state.contains_key(&bravo()));
}

/// The same case end to end: the second load finishes the split and keeps the
/// entries the first one wrote.
#[test]
fn a_crash_part_way_is_finished_by_the_next_load() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir, &combined_document());
    write_ui_state(&dir,
                   &format!(r##"{{"{ALPHA}": {{"layoutMode": "single", "splitRatio": 0.9}}}}"##));

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.workspace_ui[&alpha()].split_ratio, 0.9);
    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(WORKSPACES_FILE)).unwrap())
            .unwrap();
    for record in written.as_array().unwrap() {
        assert!(!record.as_object().unwrap().contains_key("layoutMode"),
                "the workspaces document is still combined");
    }
    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.workspace_ui, settings.workspace_ui);
}

/// Losing the UI-state document costs arrangement and nothing else - not a
/// workspace, not a name, not a colour, not an agent membership.
#[test]
fn a_missing_ui_state_document_costs_only_arrangement() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir,
                     &format!(r##"[{{"id": "{ALPHA}", "name": "Alpha",
                                    "colorHex": "#101010", "agentIds": []}}]"##));

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.saved_workspaces.len(), 1);
    assert_eq!(settings.saved_workspaces[0].name, "Alpha");
    assert_eq!(settings.saved_workspaces[0].color_hex, "#101010");
    assert_eq!(settings.workspace_ui(alpha()), WorkspaceUiState::default());
}

/// One bad entry is contained: the workspace it belongs to opens with the
/// default arrangement and its neighbours keep theirs.
#[test]
fn an_undecodable_entry_costs_only_that_workspace() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir,
                     &format!(r##"[{{"id": "{ALPHA}", "name": "Alpha", "colorHex": "#101010",
                                    "agentIds": []}},
                                  {{"id": "{BRAVO}", "name": "Bravo", "colorHex": "#202020",
                                    "agentIds": []}}]"##));
    write_ui_state(&dir,
                   &format!(r##"{{"{ALPHA}": {{"splitRatio": "not a number"}},
                                 "{BRAVO}": {{"splitRatio": 0.75}},
                                 "not-a-uuid": {{"splitRatio": 0.1}}}}"##));

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.workspace_ui(alpha()),
               WorkspaceUiState::default(),
               "the bad entry degraded to the default arrangement");
    assert_eq!(settings.workspace_ui(bravo()).split_ratio,
               0.75,
               "its neighbour kept its own");
    assert_eq!(settings.workspace_ui.len(),
               1,
               "the unkeyable entry is dropped");
}

/// Pruning is on load, against the workspaces actually present, so a deleted
/// workspace cannot leave its arrangement behind forever.
#[test]
fn an_entry_for_a_workspace_that_is_gone_is_dropped() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir,
                     &format!(r##"[{{"id": "{ALPHA}", "name": "Alpha",
                                    "colorHex": "#101010", "agentIds": []}}]"##));
    write_ui_state(&dir,
                   &format!(r##"{{"{ALPHA}": {{"splitRatio": 0.3}},
                                 "{BRAVO}": {{"splitRatio": 0.75}}}}"##));

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert!(settings.workspace_ui.contains_key(&alpha()));
    assert!(!settings.workspace_ui.contains_key(&bravo()),
            "the orphan outlived its workspace");
}

/// The two-step upgrade, which is the path nobody will try by hand: a legacy
/// `settings.json` whose `savedWorkspaces` is combined must arrive at separate
/// preferences, collection and UI-state documents with arrangement intact.
#[test]
fn a_legacy_installation_arrives_at_both_new_documents() {
    let dir = tempdir().unwrap();
    let legacy = format!(r##"{{"mcpServerPort": 9300, "savedWorkspaces": {}}}"##,
                         combined_document());
    fs::write(dir.path().join(LEGACY_SETTINGS_FILE), legacy).unwrap();

    let settings = Settings::load_from_root(dir.path()).unwrap();

    assert_eq!(settings.mcp_server_port, 9300);
    assert_eq!(settings.saved_workspaces.len(), 2);
    let ui = &settings.workspace_ui[&alpha()];
    assert_eq!(ui.layout_mode, "splitVertical");
    assert_eq!(ui.split_ratio_secondary, Some(0.4));
    assert_eq!(ui.is_detached, Some(true));
    assert_eq!(ui.window_bounds.unwrap().height, 4.0);

    // Both documents on disk, and the workspaces one narrowed.
    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(WORKSPACES_FILE)).unwrap())
            .unwrap();
    for record in written.as_array().unwrap() {
        let object = record.as_object().unwrap();
        assert!(object.contains_key("name"));
        for key in UI_KEYS {
            assert!(!object.contains_key(key),
                    "the legacy migration left {key} on the workspace record");
        }
    }
    assert!(dir.path().join(WORKSPACE_UI_STATE_FILE).exists());

    // And the launch after it reads them back unchanged.
    let reloaded = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reloaded.workspace_ui, settings.workspace_ui);
    assert_eq!(reloaded.saved_workspaces, settings.saved_workspaces);
}

/// A record carrying only some of the UI keys is still combined, and the keys
/// it lacks default rather than failing the entry.
#[test]
fn a_record_with_only_some_ui_keys_is_still_split() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir,
                     &format!(r##"[{{"id": "{ALPHA}", "name": "Alpha", "colorHex": "#101010",
                                    "agentIds": [], "windowBounds": null}}]"##));

    let split = read(&StorePaths::rooted(dir.path()));

    assert!(split.needs_write);
    assert_eq!(split.ui_state[&alpha()], WorkspaceUiState::default());
}

/// An already-split document is not a migration: `needs_write` false is what
/// keeps a load from rewriting the roster on every launch.
#[test]
fn an_already_split_document_needs_no_write() {
    let dir = tempdir().unwrap();
    write_workspaces(&dir,
                     &format!(r##"[{{"id": "{ALPHA}", "name": "Alpha",
                                    "colorHex": "#101010", "agentIds": []}}]"##));

    let split = read(&StorePaths::rooted(dir.path()));

    assert!(!split.needs_write);
    assert_eq!(split.workspaces.len(), 1);
}
