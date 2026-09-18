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
