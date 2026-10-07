//! `AgentStore::duplicate`: the copy is set up like its source and starts
//! fresh (#556).

use knot_core::{ActivationMode, CostTier, StartupPrompt};

use super::super::*;

/// A source with every setup field set away from its default, so a field
/// the duplicate dropped would read as the default and fail below.
fn configured(store: &mut AgentStore) -> Uuid {
    let id = store.create("/tmp/proj",
                          CreateOptions { name: Some("Builder".to_string()),
                                          avatar: Some("🦀".to_string()),
                                          agent_type: Some("codex".to_string()),
                                          persona_id: Some(Uuid::new_v4()),
                                          activation_mode: ActivationMode::Active,
                                          description: "Builds Knot".to_string(),
                                          capabilities: ["rust", "knot"].iter().collect(),
                                          cost_tier: CostTier::High,
                                          startup_prompt:
                                              Some(StartupPrompt::Custom("make".to_string())),
                                          ..Default::default() });
    store.set_session_config_option(id, "model".to_string(), "opus".to_string())
         .expect("the agent was just created");
    id
}

#[test]
fn a_duplicate_carries_every_setup_field() {
    let mut store = AgentStore::new();
    let source = configured(&mut store);

    let copy = store.duplicate(source).expect("the source exists");

    let (source, copy) = (store.agent(source).unwrap(), store.agent(copy).unwrap());
    assert_ne!(copy.id, source.id);
    assert_eq!(copy.avatar, source.avatar);
    assert_eq!(copy.folder, source.folder);
    assert_eq!(copy.agent_type, source.agent_type);
    assert_eq!(copy.view_mode, source.view_mode);
    assert_eq!(copy.shell_command, source.shell_command);
    assert_eq!(copy.persona_id, source.persona_id);
    assert_eq!(copy.activation_mode, source.activation_mode);
    assert_eq!(copy.startup_prompt, source.startup_prompt);
    assert_eq!(copy.session_config, source.session_config);
    // The fields #556 reported missing.
    assert_eq!(copy.description, "Builds Knot");
    assert_eq!(copy.capabilities, source.capabilities);
    assert_eq!(copy.cost_tier, CostTier::High);
}

#[test]
fn a_duplicate_is_numbered_and_placed_after_its_source() {
    let mut store = AgentStore::new();
    let source = configured(&mut store);
    let after = store.create("/tmp/other", CreateOptions::default());

    let copy = store.duplicate(source).expect("the source exists");

    assert_eq!(store.agent(copy).unwrap().name, "Builder 2");
    let order: Vec<Uuid> = store.agents().iter().map(|agent| agent.id).collect();
    assert_eq!(order, vec![source, copy, after]);
}

/// A duplicate is a new agent, not a second view of the source's session.
#[test]
fn a_duplicate_starts_without_the_sources_conversation() {
    let mut store = AgentStore::new();
    let source = configured(&mut store);
    store.set_session_id(source, "session".to_string());
    store.set_registered(source, true);

    let copy = store.duplicate(source).expect("the source exists");

    let copy = store.agent(copy).unwrap();
    assert_eq!(copy.session_id, None);
    assert!(!copy.is_registered);
    assert_eq!(copy.created_by, None);
}

#[test]
fn duplicating_a_missing_agent_creates_nothing() {
    let mut store = AgentStore::new();

    assert_eq!(store.duplicate(Uuid::new_v4()), None);
    assert!(store.agents().is_empty());
}
