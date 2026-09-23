//! Panel session config options, and the settings round-trip a removal has
//! to survive.
//!
//! `AgentStore::remove` only mutates memory, so a removal that is not
//! persisted is undone by the next launch.

use crate::app_state::build_agent_store;
use crate::tests::workspace;
use crate::workspace_window::WorkspaceWindow;

fn config_option(id: &str, category: &str) -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { id:            id.to_string(),
                             name:          id.to_string(),
                             category:      Some(category.to_string()),
                             kind:          "select".to_string(),
                             current_value: serde_json::Value::Null,
                             options:       Vec::new(), }
}

/// The case issue #194 turned on: ACP makes `category` optional, so an
/// agent may declare a perfectly good option and attach nothing to it.
fn uncategorized_option(id: &str) -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { category: None,
                             ..config_option(id, "unused") }
}

fn named_option(id: &str, name: &str) -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { name: name.to_string(),
                             ..uncategorized_option(id) }
}

#[test]
fn find_config_option_matches_category_case_insensitively() {
    let options = vec![config_option("mode", "Mode"),
                       config_option("model", "model"),];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()), Some("mode"));
}

#[test]
fn find_config_option_is_none_when_nothing_matches() {
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

/// The defect itself: ACP says a client "MUST handle missing or unknown
/// categories gracefully", and Knot required the field instead, so an
/// agent that omitted it lost all three selectors and the prompt's risk
/// colour at once.
#[test]
fn find_config_option_falls_back_to_the_id_when_there_is_no_category() {
    let options = vec![uncategorized_option("mode")];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()), Some("mode"));
}

#[test]
fn find_config_option_falls_back_to_the_id_on_an_unknown_category() {
    let options = vec![config_option("mode", "_vendor_specific")];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()), Some("mode"));
}

#[test]
fn find_config_option_falls_back_to_the_name_last() {
    let options = vec![named_option("session_behaviour", "mode")];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()),
               Some("session_behaviour"));
}

/// `mode` is a substring of `model`. Matching on anything looser than
/// equality would let the permission slot claim the model option, which is
/// why all three passes compare whole fields.
#[test]
fn find_config_option_never_matches_a_substring() {
    let options = vec![uncategorized_option("model")];

    assert!(WorkspaceWindow::find_config_option(&options, &["mode"]).is_none());
}

#[test]
fn find_config_option_ignores_non_select_options_on_every_pass() {
    let mut uncategorized = uncategorized_option("mode");
    uncategorized.kind = "boolean".to_string();
    let mut by_name = named_option("anything", "mode");
    by_name.kind = "boolean".to_string();

    assert!(WorkspaceWindow::find_config_option(&[uncategorized], &["mode"]).is_none());
    assert!(WorkspaceWindow::find_config_option(&[by_name], &["mode"]).is_none());
}

/// The test that proves this change is additive. An agent whose options
/// resolve today must keep resolving to the same one, so the category pass
/// has to finish the whole list before any fallback runs - even when an
/// earlier option would match on its id.
#[test]
fn find_config_option_prefers_a_category_match_over_an_earlier_id_match() {
    let options = vec![uncategorized_option("mode"),
                       config_option("session_behaviour", "mode"),];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()),
               Some("session_behaviour"),
               "a categorized option must outrank an earlier id match");
}

/// ACP asks clients to treat the agent's own `configOptions` order as the
/// priority order, so a tie inside one pass goes to whichever the agent
/// listed first.
#[test]
fn find_config_option_breaks_ties_by_the_agents_ordering() {
    let categorized = vec![config_option("first", "mode"),
                           config_option("second", "mode"),];
    assert_eq!(WorkspaceWindow::find_config_option(&categorized, &["mode"]).map(|option| {
                                                                               option.id.as_str()
                                                                           }),
               Some("first"));

    // Two different candidates, both matching on the id pass: the winner
    // has to be the one the agent listed first, not the one listed first
    // among the candidates.
    let uncategorized = vec![uncategorized_option("permission_mode"),
                             uncategorized_option("mode"),];
    assert_eq!(WorkspaceWindow::find_config_option(&uncategorized,
                                                   &["mode", "permission_mode"]).map(|option| {
                                                                                    option.id
                                                                                          .as_str()
                                                                                }),
               Some("permission_mode"));
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
