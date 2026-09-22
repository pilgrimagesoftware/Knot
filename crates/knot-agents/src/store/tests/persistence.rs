use std::collections::BTreeMap;

use super::super::*;

#[test]
fn restores_agents_into_a_default_workspace_when_layout_is_missing() {
    let saved = knot_core::SavedAgent::new(Uuid::new_v4(), "proj", None, "/tmp/proj");
    let id = saved.id;
    let store = AgentStore::from_saved(&[saved], Vec::new());
    assert_eq!(store.workspaces().len(), 1);
    assert_eq!(store.workspaces()[0].agent_ids, vec![id]);
    assert_eq!(store.workspaces()[0].active_agent_ids, vec![id]);
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
