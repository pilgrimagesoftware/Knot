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
fn restart_drops_the_acp_session_too() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_acp_session_id(id, "acp".to_string());
    store.restart(id).unwrap();
    assert!(store.agent(id).unwrap().acp_session_id.is_none());
}

#[test]
fn restart_keeping_conversation_keeps_the_sessions() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_session_id(id, "session".to_string());
    store.set_acp_session_id(id, "acp".to_string());
    store.set_registered(id, true);
    let token = store.agent(id).unwrap().restart_token;

    store.restart_keeping_conversation(id).unwrap();

    let agent = store.agent(id).unwrap();
    assert_eq!(agent.id, id);
    assert_ne!(agent.restart_token, token, "the session is still torn down");
    assert_eq!(agent.session_id.as_deref(), Some("session"));
    assert_eq!(agent.acp_session_id.as_deref(), Some("acp"));
    assert_eq!(agent.resume_session_id.as_deref(), Some("session"));
    assert!(!agent.is_registered);
    assert_eq!(agent.state, crate::AgentState::Idle);
}

#[test]
fn session_to_load_prefers_the_live_acp_session_over_the_resolved_one() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    assert_eq!(store.agent(id).unwrap().session_to_load(),
               None,
               "a new agent starts fresh");

    store.agent_mut(id).unwrap().resume_session_id = Some("restored".to_string());
    assert_eq!(store.agent(id).unwrap().session_to_load(), Some("restored"));

    store.set_acp_session_id(id, "live".to_string());
    assert_eq!(store.agent(id).unwrap().session_to_load(), Some("live"));

    store.restart(id).unwrap();
    assert_eq!(store.agent(id).unwrap().session_to_load(),
               None,
               "a new conversation loads neither");
}

#[test]
fn restart_keeping_conversation_clears_a_spent_fork() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.fork_session(id, "source").unwrap();
    store.restart_keeping_conversation(id).unwrap();
    assert!(!store.agent(id).unwrap().fork_session);
}

#[test]
fn restart_keeping_conversation_of_a_missing_agent_fails() {
    let mut store = AgentStore::new();
    assert!(store.restart_keeping_conversation(uuid::Uuid::new_v4())
                 .is_err());
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

/// A fork continues the source's conversation without taking it away:
/// the source keeps its own session, and the fork is marked as a fork so a
/// later restart knows to clear the flag rather than resume into it.
#[test]
fn fork_session_points_a_new_agent_at_an_existing_session() {
    let mut store = AgentStore::new();
    let source = store.create("/tmp/a", CreateOptions::default());
    store.set_session_id(source, "s1".to_string());
    let fork = store.create("/tmp/a",
                            CreateOptions { insert_after: Some(source),
                                            ..Default::default() });

    store.fork_session(fork, "s1").unwrap();

    let forked = store.agent(fork).unwrap();
    assert_eq!(forked.session_id.as_deref(), Some("s1"));
    assert_eq!(forked.resume_session_id.as_deref(), Some("s1"));
    assert!(forked.fork_session);
    assert_eq!(store.agent(source).unwrap().session_id.as_deref(),
               Some("s1"),
               "forking must not take the session from the source");
}

/// 1.3 and 2.1: creation defaults to `Passive` - the deliberate
/// disagreement with the load default a record with no stored mode gets
/// (`Active`, see `knot_core::SavedAgent::activation_mode`) - and an
/// `Active` agent is activated from birth so it starts with its workspace.
#[test]
fn create_defaults_to_passive_and_only_active_is_activated() {
    let mut store = AgentStore::new();

    let passive = store.create("/tmp/a", CreateOptions::default());
    assert_eq!(store.agent(passive).unwrap().activation_mode,
               knot_core::ActivationMode::Passive);
    assert!(!store.agent(passive).unwrap().activated);

    let active = store.create("/tmp/b",
                              CreateOptions { activation_mode:
                                                  knot_core::ActivationMode::Active,
                                              ..Default::default() });
    assert_eq!(store.agent(active).unwrap().activation_mode,
               knot_core::ActivationMode::Active);
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
    let (name, avatar, folder) = (agent.name.clone(), agent.avatar.clone(), agent.folder.clone());

    store.edit(id,
               EditRequest { name,
                             avatar,
                             folder: Some(folder),
                             agent_type: Some("claude".to_string()),
                             activation_mode: knot_core::ActivationMode::Active,
                             ..Default::default() })
         .unwrap();

    assert_eq!(store.agent(id).unwrap().activation_mode,
               knot_core::ActivationMode::Active);
    assert_eq!(store.agent(id).unwrap().restart_token, token);
}

/// 2.3: deactivation is not removal. The owner and its companions lose
/// only their `activated` flag - they keep their place in the agent list,
/// their ordering, and their workspace membership - and the cascade hands
/// companions back before their owner, the order `remove` uses.
#[test]
fn deactivating_an_owner_cascades_without_removing_anything() {
    let mut store = AgentStore::new();
    let owner = store.create("/tmp/a",
                             CreateOptions { activation_mode: knot_core::ActivationMode::Active,
                                             ..Default::default() });
    let first = store.create_shell_companion(owner).unwrap();
    let second = store.create_shell_companion(owner).unwrap();
    let other = store.create("/tmp/b",
                             CreateOptions { activation_mode: knot_core::ActivationMode::Active,
                                             ..Default::default() });
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
    assert_eq!(store.agents()
                    .iter()
                    .map(|agent| agent.id)
                    .collect::<Vec<_>>(),
               order);
    assert_eq!(store.workspaces()[0].agent_ids, workspace);
    assert_eq!(store.agent(owner).unwrap().activation_mode,
               knot_core::ActivationMode::Active);
}

/// `agent-registry` - "Registry metadata survives the bench round trip".
/// A template that records only a folder is not enough to choose from a
/// registry, so deployment must restore the role too.
#[test]
fn deploying_a_bench_entry_restores_its_registry_metadata() {
    let mut store = AgentStore::default();
    let mut bench = knot_core::BenchAgent::new(Uuid::new_v4(), "Reviewer", None, "/repo");
    bench.description = "Reviews Rust diffs".to_string();
    bench.capabilities = ["rust", "code-review"].iter().collect();
    bench.cost_tier = knot_core::CostTier::High;

    let id = store.deploy_bench(&bench, |_| true).expect("folder exists");

    let agent = store.agent(id).expect("deployed");
    assert_eq!(agent.description, "Reviews Rust diffs");
    assert!(agent.capabilities.contains("rust"));
    assert!(agent.capabilities.contains("code-review"));
    assert_eq!(agent.cost_tier, knot_core::CostTier::High);
}

/// `agent-lifecycle` - "Re-tagging a working agent does not interrupt it".
#[test]
fn editing_only_registry_metadata_does_not_restart() {
    let mut store = AgentStore::default();
    let id = store.create("/repo", CreateOptions::default());
    store.agent_mut(id).unwrap().state = AgentState::Running;
    let token = store.agent(id).unwrap().restart_token;
    let agent = store.agent(id).unwrap();
    let req = EditRequest { name: agent.name.clone(),
                            avatar: agent.avatar.clone(),
                            description: "Runs the test suite".to_string(),
                            capabilities: ["testing"].iter().collect(),
                            cost_tier: knot_core::CostTier::Low,
                            ..Default::default() };

    store.edit(id, req).expect("edit succeeds");

    let agent = store.agent(id).unwrap();
    assert_eq!(agent.restart_token, token,
               "a re-tag must not recreate the session");
    assert_eq!(agent.state,
               AgentState::Running,
               "and must not interrupt the work");
    assert_eq!(agent.description, "Runs the test suite");
    assert!(agent.capabilities.contains("testing"));
    assert_eq!(agent.cost_tier, knot_core::CostTier::Low);
}
