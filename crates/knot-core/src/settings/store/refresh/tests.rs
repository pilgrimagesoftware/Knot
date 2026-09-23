//! What a refresh is allowed to change, and what it must not touch.
//!
//! The second of those is the one worth testing. Adopting a scalar is one
//! `serde` decode and would be hard to get wrong; silently emptying a
//! collection is a data loss the user would discover later, somewhere else.

use std::collections::BTreeMap;

use tempfile::tempdir;
use uuid::Uuid;

use crate::settings::records::BenchAgent;
use crate::settings::records::Persona;
use crate::settings::records::PersonaState;
use crate::settings::records::PersonaType;
use crate::settings::records::SavedAgent;
use crate::settings::records::SavedPullRequest;
use crate::settings::records::Workspace;
use crate::settings::records::WorkspaceUiState;
use crate::settings::store::Settings;

/// Every `#[serde(skip)]` collection, filled with one recognizable value.
///
/// Deliberately exhaustive rather than representative: this is the list
/// `Settings::reload_preferences` transplants by hand, so a field added to
/// the struct and forgotten there is exactly what this should catch.
fn with_every_collection_filled(settings: &mut Settings) {
    let workspace_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();

    settings.saved_agents = vec![SavedAgent::new(agent_id, "Kept", None, "/repo")];
    settings.saved_workspaces = vec![Workspace { id:        workspace_id,
                                                 name:      "Kept".to_string(),
                                                 color_hex: "#1B4FB2".to_string(),
                                                 agent_ids: vec![agent_id], }];
    settings.workspace_ui = BTreeMap::from([(workspace_id, WorkspaceUiState::default())]);
    settings.personas = vec![Persona { id:           Uuid::new_v4(),
                                       name:         "Kept".to_string(),
                                       instructions: String::new(),
                                       persona_type: PersonaType::User,
                                       state:        PersonaState::Enabled, }];
    settings.bench_agents = vec![BenchAgent::new(Uuid::new_v4(), "Kept", None, "/repo")];
    settings.recent_repos = vec!["/repo".to_string()];
    settings.pull_requests =
        vec![SavedPullRequest::new("https://example.test/pr/1", agent_id, workspace_id)];
}

/// The defect this whole change is about: a value copied before the write
/// has to see the write.
#[test]
fn reload_preferences_adopts_a_scalar_written_after_the_copy_was_taken() {
    let dir = tempdir().unwrap();

    let mut window_copy = Settings::with_store_root(dir.path());
    window_copy.agent_panel_compact_tool_calls = false;
    window_copy.persist_preferences().unwrap();

    // Another holder of the same store turns the preference on, as the
    // settings window does.
    let mut settings_window = Settings::with_store_root(dir.path());
    settings_window.agent_panel_compact_tool_calls = true;
    settings_window.mcp_server_port = 9100;
    settings_window.persist_preferences().unwrap();

    window_copy.reload_preferences().unwrap();

    assert!(window_copy.agent_panel_compact_tool_calls,
            "the refreshed copy is still drawing the preference it was born with");
    assert_eq!(window_copy.mcp_server_port, 9100,
               "every scalar comes from the document, not only the one under test");
}

/// The hazard: the collections live in their own documents, and the holder's
/// may be ahead of them. A refresh that reached for those too would be the
/// read-side twin of #238's write-side lost update.
#[test]
fn reload_preferences_keeps_every_collection() {
    let dir = tempdir().unwrap();

    let mut settings = Settings::with_store_root(dir.path());
    settings.persist_preferences().unwrap();
    with_every_collection_filled(&mut settings);

    settings.reload_preferences().unwrap();

    assert_eq!(settings.saved_agents.len(),
               1,
               "saved_agents was cleared by a refresh");
    assert_eq!(settings.saved_workspaces.len(),
               1,
               "saved_workspaces was cleared by a refresh");
    assert_eq!(settings.workspace_ui.len(),
               1,
               "workspace_ui was cleared by a refresh");
    assert_eq!(settings.personas.len(),
               1,
               "personas was cleared by a refresh");
    assert_eq!(settings.bench_agents.len(),
               1,
               "bench_agents was cleared by a refresh");
    assert_eq!(settings.recent_repos.len(),
               1,
               "recent_repos was cleared by a refresh");
    assert_eq!(settings.pull_requests.len(),
               1,
               "pull_requests was cleared by a refresh");
}

/// The refreshed value has to stay able to write, or the next preference
/// change from that window would go to the platform directory instead of the
/// store it was rooted at - which in a test is the developer's own settings.
#[test]
fn reload_preferences_keeps_the_store_root() {
    let dir = tempdir().unwrap();
    let mut settings = Settings::with_store_root(dir.path());
    settings.mcp_server_port = 9200;
    settings.persist_preferences().unwrap();

    settings.reload_preferences().unwrap();
    settings.mcp_server_port = 9300;
    settings.persist_preferences().unwrap();

    let reread = Settings::load_from_root(dir.path()).unwrap();
    assert_eq!(reread.mcp_server_port, 9300,
               "the refreshed value wrote somewhere other than its own store root");
}

/// A refresh runs on a user action in a running app. A store with no
/// preferences document yet - a first launch that has changed nothing - must
/// not turn that action into an error.
#[test]
fn reload_preferences_survives_a_store_with_no_preferences_document() {
    let dir = tempdir().unwrap();
    let mut settings = Settings::with_store_root(dir.path());
    with_every_collection_filled(&mut settings);

    settings.reload_preferences()
            .expect("a missing preferences document is a default, not a failure");

    assert_eq!(settings.saved_agents.len(),
               1,
               "the collections went missing along with the document");
}
