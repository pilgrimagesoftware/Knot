use super::super::*;

#[test]
fn companion_is_bound_and_cannot_own_companions() {
    let mut store = AgentStore::new();
    let owner = store.create("/tmp/a", CreateOptions::default());
    let companion = store.create_shell_companion(owner).unwrap();
    assert_eq!(store.agent(companion).unwrap().created_by, Some(owner));
    assert!(store.agent(companion).unwrap().is_companion);
    assert_eq!(
        store.create_shell_companion(companion).unwrap_err(),
        crate::error::AgentError::CompanionCannotOwn(companion)
    );
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
    store
        .edit(
            owner,
            EditRequest {
                name: "owner".to_string(),
                avatar: String::new(),
                folder: Some("/tmp/new".to_string()),
                relocate_companions: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_ne!(store.agent(owner).unwrap().restart_token, owner_token);
    assert_eq!(store.agent(companion).unwrap().folder, "/tmp/new");
}

/// A fork continues the source's conversation without taking it away:
/// the source keeps its own session, and the fork is marked as a fork so a
/// later restart knows to clear the flag rather than resume into it.
#[test]
fn fork_session_points_a_new_agent_at_an_existing_session() {
    let mut store = AgentStore::new();
    let source = store.create("/tmp/a", CreateOptions::default());
    store.set_session_id(source, "s1".to_string());
    let fork = store.create(
        "/tmp/a",
        CreateOptions {
            insert_after: Some(source),
            ..Default::default()
        },
    );

    store.fork_session(fork, "s1").unwrap();

    let forked = store.agent(fork).unwrap();
    assert_eq!(forked.session_id.as_deref(), Some("s1"));
    assert_eq!(forked.resume_session_id.as_deref(), Some("s1"));
    assert!(forked.fork_session);
    assert_eq!(
        store.agent(source).unwrap().session_id.as_deref(),
        Some("s1"),
        "forking must not take the session from the source"
    );
}

/// 1.3 and 2.1: creation defaults to `Passive` - the deliberate
/// disagreement with the load default a record with no stored mode gets
/// (`Active`, see `knot_core::SavedAgent::activation_mode`) - and an
/// `Active` agent is activated from birth so it starts with its workspace.
#[test]
fn create_defaults_to_passive_and_only_active_is_activated() {
    let mut store = AgentStore::new();

    let passive = store.create("/tmp/a", CreateOptions::default());
    assert_eq!(
        store.agent(passive).unwrap().activation_mode,
        knot_core::ActivationMode::Passive
    );
    assert!(!store.agent(passive).unwrap().activated);

    let active = store.create(
        "/tmp/b",
        CreateOptions {
            activation_mode: knot_core::ActivationMode::Active,
            ..Default::default()
        },
    );
    assert_eq!(
        store.agent(active).unwrap().activation_mode,
        knot_core::ActivationMode::Active
    );
    assert!(store.agent(active).unwrap().activated);
}

#[test]
fn set_activated_toggles_the_runtime_flag() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());

    store.set_activated(id, true);
    assert!(store.agent(id).unwrap().activated);

    store.set_activated(id, false);
    assert!(!store.agent(id).unwrap().activated);
}

/// 1.4: changing only the activation mode is not a restart-worthy edit.
#[test]
fn editing_only_the_activation_mode_does_not_restart() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let token = store.agent(id).unwrap().restart_token;
    let agent = store.agent(id).unwrap();
    let (name, avatar, folder) = (
        agent.name.clone(),
        agent.avatar.clone(),
        agent.folder.clone(),
    );

    store
        .edit(
            id,
            EditRequest {
                name,
                avatar,
                folder: Some(folder),
                agent_type: Some("claude".to_string()),
                activation_mode: knot_core::ActivationMode::Active,
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(
        store.agent(id).unwrap().activation_mode,
        knot_core::ActivationMode::Active
    );
    assert_eq!(store.agent(id).unwrap().restart_token, token);
}

/// 2.3: deactivation is not removal. The owner and its companions lose
/// only their `activated` flag - they keep their place in the agent list,
/// their ordering, and their workspace membership - and the cascade hands
/// companions back before their owner, the order `remove` uses.
#[test]
fn deactivating_an_owner_cascades_without_removing_anything() {
    let mut store = AgentStore::new();
    let owner = store.create(
        "/tmp/a",
        CreateOptions {
            activation_mode: knot_core::ActivationMode::Active,
            ..Default::default()
        },
    );
    let first = store.create_shell_companion(owner).unwrap();
    let second = store.create_shell_companion(owner).unwrap();
    let other = store.create(
        "/tmp/b",
        CreateOptions {
            activation_mode: knot_core::ActivationMode::Active,
            ..Default::default()
        },
    );
    let order: Vec<_> = store.agents().iter().map(|agent| agent.id).collect();
    let workspace = store.workspaces()[0].agent_ids.clone();

    let deactivated = store.deactivate(owner);

    assert_eq!(deactivated.last(), Some(&owner));
    assert!(deactivated.contains(&first) && deactivated.contains(&second));
    for id in [owner, first, second] {
        assert!(!store.agent(id).unwrap().activated);
    }
    // An unrelated agent is untouched, and nothing moved or vanished.
    assert!(store.agent(other).unwrap().activated);
    assert_eq!(
        store
            .agents()
            .iter()
            .map(|agent| agent.id)
            .collect::<Vec<_>>(),
        order
    );
    assert_eq!(store.workspaces()[0].agent_ids, workspace);
    assert_eq!(
        store.agent(owner).unwrap().activation_mode,
        knot_core::ActivationMode::Active
    );
}
