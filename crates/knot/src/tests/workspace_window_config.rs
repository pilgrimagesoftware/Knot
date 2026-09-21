//! Panel session config options, and the settings round-trip a removal has
//! to survive.
//!
//! `AgentStore::remove` only mutates memory, so a removal that is not
//! persisted is undone by the next launch.

use super::*;

fn config_option(id: &str, category: &str) -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { id:            id.to_string(),
                             name:          id.to_string(),
                             category:      Some(category.to_string()),
                             kind:          "select".to_string(),
                             current_value: serde_json::Value::Null,
                             options:       Vec::new(), }
}

#[test]
fn find_config_option_matches_category_case_insensitively() {
    let options = vec![config_option("mode", "Mode"),
                       config_option("model", "model"),];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()), Some("mode"));
}

#[test]
fn find_config_option_is_none_when_no_category_matches() {
    let options = vec![config_option("mode", "mode")];

    assert!(WorkspaceWindow::find_config_option(&options, &["model"]).is_none());
}

#[test]
fn find_config_option_ignores_non_select_options() {
    let mut boolean_option = config_option("brave_mode", "mode");
    boolean_option.kind = "boolean".to_string();
    let options = vec![boolean_option];

    assert!(WorkspaceWindow::find_config_option(&options, &["mode"]).is_none());
}

/// Regression guard for "Remove Agent does nothing": `AgentStore::remove`
/// only mutates memory, and nothing writes settings on quit, so unless the
/// caller re-snapshots `saved_agents`/`saved_workspaces` afterwards the
/// removal is undone by the next launch. `WorkspaceWindow::remove_agent`
/// does that snapshot; this pins the round trip it depends on.
#[test]
fn removing_an_agent_survives_a_settings_round_trip() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let doomed = store.create("~/doomed", knot_agents::CreateOptions::default());
    let kept = store.create("~/kept", knot_agents::CreateOptions::default());

    assert_eq!(store.remove(doomed).len(), 1);

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.saved_agents = store.saved_agents(false);
    settings.saved_workspaces = store.saved_workspaces();

    let restored = build_agent_store(&settings);
    assert!(restored.agent(doomed).is_none(),
            "a removed agent must not come back after a restore");
    assert!(restored.agent(kept).is_some());
    assert_eq!(restored.workspaces()[0].agent_ids, vec![kept]);
}

/// Removing an agent takes its shell companions with it, so the persisted
/// snapshot must lose them too rather than leaving orphans behind.
#[test]
fn removing_an_agent_also_drops_its_companions_from_the_snapshot() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let parent = store.create("~/parent", knot_agents::CreateOptions::default());
    store.create_shell_companion(parent)
         .expect("companion should be creatable");
    assert_eq!(store.agents().len(), 2);

    assert_eq!(store.remove(parent).len(), 2);

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.saved_agents = store.saved_agents(false);
    settings.saved_workspaces = store.saved_workspaces();

    assert!(build_agent_store(&settings).agents().is_empty());
}
