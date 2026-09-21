//! Restoring the roster at launch: which agents come back, whether their
//! conversations resume, and which one is selected.

use super::*;

#[test]
fn build_agent_store_restores_layout_when_enabled() {
    let agent_id = Uuid::new_v4();
    let saved = knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha");
    let mut ws = workspace("Restored");
    ws.agent_ids = vec![agent_id];

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.saved_agents = vec![saved];
    settings.saved_workspaces = vec![ws.clone()];

    let store = build_agent_store(&settings);
    assert_eq!(store.agents().len(), 1);
    assert_eq!(store.workspaces(), &[ws.clone()]);
    assert_eq!(store.current_workspace_id(), Some(ws.id));
}

#[test]
fn build_agent_store_restores_exact_session_id_when_conversation_enabled() {
    let agent_id = Uuid::new_v4();
    let mut saved = knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha");
    saved.session_id = Some("s7".to_string());

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.restore_conversation_on_launch = true;
    settings.saved_agents = vec![saved];

    let store = build_agent_store(&settings);
    let agent = store.agent(agent_id).unwrap();
    assert_eq!(agent.resume_session_id.as_deref(), Some("s7"));
    assert!(agent.session_id.is_none());
}

#[test]
fn build_agent_store_leaves_resume_session_unset_when_conversation_disabled() {
    let agent_id = Uuid::new_v4();
    let mut saved = knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha");
    saved.session_id = Some("s7".to_string());

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.restore_conversation_on_launch = false;
    settings.saved_agents = vec![saved];

    let store = build_agent_store(&settings);
    let agent = store.agent(agent_id).unwrap();
    assert!(agent.resume_session_id.is_none());
}

#[test]
fn build_agent_store_starts_empty_when_restore_disabled() {
    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = false;
    settings.saved_agents =
        vec![knot_core::SavedAgent::new(Uuid::new_v4(), "alpha", None, "~/alpha")];

    let store = build_agent_store(&settings);
    assert!(store.agents().is_empty());
    assert!(store.workspaces().is_empty());
}

#[test]
fn initial_selection_prefers_active_agent_and_skips_stale_ids() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let first = store.create("~/first", knot_agents::CreateOptions::default());
    let second = store.create("~/second", knot_agents::CreateOptions::default());

    let mut saved = store.saved_workspaces()[0].clone();
    saved.active_agent_ids = vec![Uuid::new_v4(), second];
    let restored = knot_agents::AgentStore::from_saved(&store.saved_agents(false), vec![saved]);

    assert_eq!(initial_agent_selection(&restored), Some(second));
    assert_ne!(initial_agent_selection(&restored), Some(first));
}
