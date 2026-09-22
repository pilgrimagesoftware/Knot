//! The agent editor's registry controls: how the capabilities field is
//! read, and that the three fields survive the edit path without
//! restarting the agent.
//!
//! Pins `openspec/specs/agent-registry/spec.md` and `agent-lifecycle`'s
//! "Edit triggers restart only for launch-affecting changes".

use knot_agents::{AgentState, AgentStore, CreateOptions, EditRequest};
use knot_core::CostTier;

use crate::agent_editor::parse_capability_tags;

#[test]
fn the_capabilities_field_splits_on_commas_and_normalizes() {
    let tags = parse_capability_tags(" Rust , testing ,, CODE-REVIEW ");

    assert_eq!(tags.len(), 3);
    assert!(tags.contains("rust"));
    assert!(tags.contains("testing"));
    assert!(tags.contains("code-review"));
}

#[test]
fn an_empty_capabilities_field_is_no_tags() {
    assert!(parse_capability_tags("").is_empty());
    assert!(parse_capability_tags("   ,  , ").is_empty());
}

/// A single tag with no comma is the common case and must not need one.
#[test]
fn one_tag_needs_no_comma() {
    let tags = parse_capability_tags("infrastructure");

    assert_eq!(tags.len(), 1);
    assert!(tags.contains("infrastructure"));
}

/// The form now owns these fields, so saving it must carry what was typed
/// - and must not interrupt an agent that is working.
#[test]
fn saving_registry_fields_changes_them_without_restarting() {
    let mut store = AgentStore::new();
    let id = store.create("/repo", CreateOptions::default());
    store.set_state(id, AgentState::Running);
    let token = store.agent(id).unwrap().restart_token;

    store.edit(id,
               EditRequest { name: "worker".to_string(),
                             description: "Runs the test suite".to_string(),
                             capabilities: parse_capability_tags("testing, rust"),
                             cost_tier: CostTier::Low,
                             ..Default::default() })
         .expect("edit succeeds");

    let agent = store.agent(id).unwrap();
    assert_eq!(agent.description, "Runs the test suite");
    assert!(agent.capabilities.contains("testing"));
    assert!(agent.capabilities.contains("rust"));
    assert_eq!(agent.cost_tier, CostTier::Low);
    assert_eq!(agent.restart_token, token,
               "a metadata edit must not restart");
    assert_eq!(agent.state, AgentState::Running);
}
