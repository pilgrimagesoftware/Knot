use super::super::*;

#[test]
fn companion_is_bound_and_cannot_own_companions() {
    let mut store = AgentStore::new();
    let owner = store.create("/tmp/a", CreateOptions::default());
    let companion = store.create_shell_companion(owner).unwrap();
    assert_eq!(store.agent(companion).unwrap().created_by, Some(owner));
    assert!(store.agent(companion).unwrap().is_companion);
    assert_eq!(store.create_shell_companion(companion).unwrap_err(),
               crate::error::AgentError::CompanionCannotOwn(companion));
}

#[test]
fn removing_an_owner_cascades_to_companions() {
    let mut store = AgentStore::new();
    let owner = store.create("/tmp/a", CreateOptions::default());
    let companion = store.create_shell_companion(owner).unwrap();
    let removed = store.remove(owner);
    assert_eq!(removed.last().map(|agent| agent.id), Some(owner));
    assert!(removed.iter().any(|agent| agent.id == companion));
    assert!(store.agents().is_empty());
}

#[test]
fn restart_keeps_identity_and_drops_session() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_session_id(id, "session".to_string());
    let token = store.agent(id).unwrap().restart_token;
    store.restart(id).unwrap();
    let agent = store.agent(id).unwrap();
    assert_eq!(agent.id, id);
    assert_ne!(agent.restart_token, token);
    assert!(agent.session_id.is_none());
    assert!(!agent.is_registered);
}

#[test]
fn edit_restarts_folder_changes_and_relocates_companions() {
    let mut store = AgentStore::new();
    let owner = store.create("/tmp/old", CreateOptions::default());
    let companion = store.create_shell_companion(owner).unwrap();
    let owner_token = store.agent(owner).unwrap().restart_token;
    store.edit(owner,
               EditRequest { name: "owner".to_string(),
                             avatar: String::new(),
                             folder: Some("/tmp/new".to_string()),
                             relocate_companions: true,
                             ..Default::default() })
         .unwrap();
    assert_ne!(store.agent(owner).unwrap().restart_token, owner_token);
    assert_eq!(store.agent(companion).unwrap().folder, "/tmp/new");
}
