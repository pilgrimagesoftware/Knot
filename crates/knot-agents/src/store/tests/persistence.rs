use std::collections::BTreeMap;

use super::super::*;

/// A workspace holding `agent_ids`, for the adoption tests below.
fn workspace_named(name: &str, agent_ids: Vec<Uuid>) -> knot_core::Workspace {
    knot_core::Workspace { id: Uuid::new_v4(),
                           name: name.to_string(),
                           color_hex: "#000000".to_string(),
                           agent_ids }
}

#[test]
fn restores_agents_into_a_default_workspace_when_layout_is_missing() {
    let saved = knot_core::SavedAgent::new(Uuid::new_v4(), "proj", None, "/tmp/proj");
    let id = saved.id;
    let store = AgentStore::from_saved(&[saved], Vec::new());
    assert_eq!(store.workspaces().len(), 1);
    assert_eq!(store.workspaces()[0].agent_ids, vec![id]);
    let workspace_id = store.workspaces()[0].id;
    assert_eq!(store.workspace_ui(workspace_id).active_agent_ids, vec![id]);
}

#[test]
fn resume_resolution_prefers_persisted_id_and_falls_back() {
    let saved = knot_core::SavedAgent::new(Uuid::new_v4(), "proj", None, "/tmp/proj");
    let id = saved.id;
    let mut store = AgentStore::from_saved(&[saved], Vec::new());
    store.resolve_resume_sessions(&BTreeMap::from([(id, "saved".to_string())]), |_, _| {
             Some("fallback".to_string())
         });
    assert_eq!(store.agent(id).unwrap().resume_session_id.as_deref(),
               Some("saved"));

    let saved = knot_core::SavedAgent::new(Uuid::new_v4(), "proj", None, "/tmp/proj");
    let id = saved.id;
    let mut store = AgentStore::from_saved(&[saved], Vec::new());
    store.resolve_resume_sessions(&BTreeMap::new(), |folder, agent_type| {
             assert_eq!(folder, "/tmp/proj");
             assert_eq!(agent_type, "claude");
             Some("fallback".to_string())
         });
    assert_eq!(store.agent(id).unwrap().resume_session_id.as_deref(),
               Some("fallback"));
}

/// 4.2: a settings file written before activation mode existed must behave
/// exactly as it did - every agent in it starts when its workspace opens.
/// The record's missing `activationMode` loads as `Active`, and the
/// workspace-open pass turns that into the runtime flag `ensure_*` gates on.
#[test]
fn a_legacy_settings_file_still_starts_every_agent_on_open() {
    let json = format!(r#"[{{"id":"{}","name":"A","avatar":"x","folder":"/tmp/a"}},
                           {{"id":"{}","name":"B","avatar":"y","folder":"/tmp/b"}}]"#,
                       Uuid::new_v4(),
                       Uuid::new_v4());
    let saved: Vec<knot_core::SavedAgent> = serde_json::from_str(&json).unwrap();
    let mut store = AgentStore::from_saved(&saved, Vec::new());
    let ids = store.agents()
                   .iter()
                   .map(|agent| agent.id)
                   .collect::<Vec<_>>();

    // Loading alone activates nothing - `activated` is runtime-only.
    assert!(store.agents().iter().all(|agent| !agent.activated));

    let activated = store.activate_on_workspace_open(&ids);

    assert_eq!(activated, ids);
    assert!(store.agents().iter().all(|agent| agent.activated));
}

/// 4.1: restoring a layout must not start a passive agent. The restore path
/// sets a selection without going through activation, so the only thing
/// that runs on open is this pass - and it leaves `Passive` agents alone.
#[test]
fn opening_a_workspace_starts_only_its_active_agents() {
    let mut store = AgentStore::new();
    let active = store.create("/tmp/a",
                              CreateOptions { activation_mode:
                                                  knot_core::ActivationMode::Active,
                                              ..Default::default() });
    let passive = store.create("/tmp/b", CreateOptions::default());
    // Reload, the way a relaunch does: every runtime flag resets.
    let saved = store.saved_agents(false);
    let workspaces = store.saved_workspaces();
    let mut store = AgentStore::from_saved(&saved, workspaces);

    let activated = store.activate_on_workspace_open(&[active, passive]);

    assert_eq!(activated, vec![active]);
    assert!(store.agent(active).unwrap().activated);
    assert!(!store.agent(passive).unwrap().activated,
            "a passive agent stays stopped across a relaunch");
}

/// Deactivation lasts as long as the workspace stays open: nothing
/// re-runs the open pass while it is open, and reopening it is a new
/// session for an `active` agent by definition.
#[test]
fn deactivating_survives_until_the_workspace_is_reopened() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a",
                          CreateOptions { activation_mode: knot_core::ActivationMode::Active,
                                          ..Default::default() });

    store.deactivate(id);
    assert!(!store.agent(id).unwrap().activated);

    store.activate_on_workspace_open(&[id]);
    assert!(store.agent(id).unwrap().activated);
}

/// `session-setup-persistence`: a selection recorded mid-session changes
/// only the stored setup. Nothing about the turn in flight - its state, its
/// session ids - is touched, so the running turn keeps the setup it started
/// with and the new value is what the next session replays.
#[test]
fn recording_a_setup_selection_leaves_the_running_turn_alone() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_session_id(id, "term-1".to_string());
    store.set_acp_session_id(id, "acp-1".to_string());
    store.set_state(id, AgentState::Running);

    store.set_session_config_option(id, "model".to_string(), "opus".to_string())
         .unwrap();

    let agent = store.agent(id).unwrap();
    assert_eq!(agent.state, AgentState::Running, "the turn keeps running");
    assert_eq!(agent.session_id.as_deref(), Some("term-1"));
    assert_eq!(agent.acp_session_id.as_deref(), Some("acp-1"));
    assert_eq!(agent.session_config.get("model").map(String::as_str),
               Some("opus"));
}

/// The recorded setup is what a later launch replays: it reaches the saved
/// record, and `from_saved` puts it back on the runtime agent.
#[test]
fn a_recorded_setup_survives_save_and_reload() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_session_config_option(id, "model".to_string(), "opus".to_string())
         .unwrap();
    store.set_session_config_option(id, "permission_mode".to_string(), "plan".to_string())
         .unwrap();

    let saved = store.saved_agents(false);
    let reloaded = AgentStore::from_saved(&saved, Vec::new());

    assert_eq!(reloaded.session_config(id),
               BTreeMap::from([("model".to_string(), "opus".to_string()),
                               ("permission_mode".to_string(), "plan".to_string())]));
}

/// An agent saved before the field existed replays nothing, leaving the
/// adapter's own defaults in place.
#[test]
fn a_legacy_agent_replays_an_empty_setup() {
    let saved = knot_core::SavedAgent::new(Uuid::new_v4(), "proj", None, "/tmp/proj");
    let id = saved.id;
    let store = AgentStore::from_saved(&[saved], Vec::new());
    assert!(store.session_config(id).is_empty());
}

/// An import adds records the store has never seen. They must arrive with the
/// identifiers they already have: the import's own skip-if-present check is
/// keyed on them, and a re-import would otherwise duplicate everything.
#[test]
fn adopting_saved_records_keeps_their_identifiers() {
    let mut store = AgentStore::new();
    let agent = knot_core::SavedAgent::new(Uuid::new_v4(), "imported", None, "/tmp/imported");
    let workspace = workspace_named("SRE", vec![agent.id]);

    let adopted = store.adopt_saved(std::slice::from_ref(&agent),
                                    std::slice::from_ref(&workspace));

    assert_eq!(adopted.agents, 1);
    assert_eq!(adopted.workspaces, 1);
    assert_eq!(store.agent(agent.id).map(|a| a.name.as_str()),
               Some("imported"));
    assert_eq!(store.workspaces().iter().map(|w| w.id).collect::<Vec<_>>(),
               vec![workspace.id]);
}

/// The defect this exists for: an import wrote to settings while the store
/// went on holding what it loaded at startup, so the next store-to-settings
/// write put the old list back over the imported one.
#[test]
fn an_adopted_workspace_survives_a_round_trip_back_to_settings() {
    let mut store = AgentStore::new();
    let agent = knot_core::SavedAgent::new(Uuid::new_v4(), "imported", None, "/tmp/imported");
    let workspace = workspace_named("SRE", vec![agent.id]);

    store.adopt_saved(&[agent], std::slice::from_ref(&workspace));

    let written_back = store.saved_workspaces();
    assert!(written_back.iter().any(|w| w.id == workspace.id),
            "the adopted workspace is missing from what the store writes back to settings");
}

/// The live copy wins: an agent the store already holds has session state an
/// incoming record does not, so adopting must not replace it.
#[test]
fn adopting_a_record_the_store_already_holds_leaves_the_live_one_alone() {
    let id = Uuid::new_v4();
    let held = knot_core::SavedAgent::new(id, "live", None, "/tmp/live");
    let mut store = AgentStore::from_saved(&[held], vec![workspace_named("Held", vec![id])]);
    let incoming = knot_core::SavedAgent::new(id, "stale copy", None, "/tmp/elsewhere");

    let adopted = store.adopt_saved(&[incoming], &[]);

    assert_eq!(adopted.agents, 0, "an agent already held was adopted again");
    assert_eq!(store.agents().len(), 1);
    assert_eq!(store.agent(id).map(|a| a.name.as_str()), Some("live"));
}

/// Adopting twice is what a user re-running an import does.
#[test]
fn adopting_twice_adds_nothing_the_second_time() {
    let mut store = AgentStore::new();
    let agent = knot_core::SavedAgent::new(Uuid::new_v4(), "imported", None, "/tmp/imported");
    let workspace = workspace_named("SRE", vec![agent.id]);

    store.adopt_saved(std::slice::from_ref(&agent),
                      std::slice::from_ref(&workspace));
    let again = store.adopt_saved(&[agent], &[workspace]);

    assert_eq!(again.agents, 0);
    assert_eq!(again.workspaces, 0);
    assert_eq!(store.agents().len(), 1);
    assert_eq!(store.workspaces().len(), 1);
}

/// A store that held nothing has no current workspace, and a window with none
/// selected renders an empty list however many were just adopted.
#[test]
fn adopting_into_an_empty_store_selects_a_current_workspace() {
    let mut store = AgentStore::new();
    let workspace = workspace_named("SRE", Vec::new());

    store.adopt_saved(&[], std::slice::from_ref(&workspace));

    assert_eq!(store.current_workspace_id(), Some(workspace.id));
}
