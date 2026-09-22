//! The sidebar's layout model: which rows a store produces, which are
//! marked selected, and what each row carries.

use std::collections::BTreeMap;

use uuid::Uuid;

use crate::app_state::agent_selection_for_workspace;
use crate::app_state::layout_model;
use crate::app_state::state_label;
use crate::tests::workspace;
use crate::workspace_window::runs_a_terminal_process;

#[test]
fn empty_store_has_no_rows() {
    let store = knot_agents::AgentStore::new();
    let model = layout_model(&store, None, &[], &BTreeMap::new());
    assert!(model.workspace_rows.is_empty());
    assert!(model.selected_agent_rows.is_empty());
}

#[test]
fn selected_workspace_marks_and_filters_rows() {
    let mut store = knot_agents::AgentStore::new();
    let ws1 = workspace("One");
    let ws2 = workspace("Two");
    store.add_workspace(ws1.clone());
    store.add_workspace(ws2.clone());

    store.set_current_workspace(ws1.id);
    store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());

    store.set_current_workspace(ws2.id);
    store.create("~/gamma", knot_agents::CreateOptions::default());

    store.set_current_workspace(ws1.id);
    let model = layout_model(&store, None, &[], &BTreeMap::new());

    assert_eq!(model.workspace_rows.len(), 2);
    assert!(model.workspace_rows
                 .iter()
                 .find(|r| r.id == ws1.id)
                 .unwrap()
                 .selected);
    assert!(!model.workspace_rows
                  .iter()
                  .find(|r| r.id == ws2.id)
                  .unwrap()
                  .selected);

    let names = model.selected_agent_rows
                     .iter()
                     .map(|r| r.name.as_str())
                     .collect::<Vec<_>>();
    assert_eq!(names, vec!["alpha", "beta"]);
    assert!(!names.contains(&"gamma"));
    let gamma_id = store.agents()
                        .iter()
                        .find(|agent| agent.name == "gamma")
                        .map(|agent| agent.id)
                        .unwrap();
    assert_eq!(agent_selection_for_workspace(&store, ws2.id),
               Some(gamma_id));
}

#[test]
fn missing_agent_ids_are_skipped() {
    let mut store = knot_agents::AgentStore::new();
    let mut ws = workspace("One");
    ws.agent_ids.push(Uuid::new_v4());
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    store.create("~/alpha", knot_agents::CreateOptions::default());

    let model = layout_model(&store, None, &[], &BTreeMap::new());
    assert_eq!(model.selected_agent_rows.len(), 1);
    assert_eq!(model.selected_agent_rows[0].name, "alpha");
}

#[test]
fn agent_selection_marks_and_tracks_attach_state() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let alpha_id = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());

    let model = layout_model(&store, Some(alpha_id), &[], &BTreeMap::new());
    let alpha = model.selected_agent_rows
                     .iter()
                     .find(|row| row.id == alpha_id)
                     .unwrap();
    assert!(alpha.selected);
    assert!(!alpha.attached);
    assert_eq!(alpha.state, knot_agents::AgentState::Idle);
    let beta = model.selected_agent_rows
                    .iter()
                    .find(|row| row.id != alpha_id)
                    .unwrap();
    assert!(!beta.selected);

    let model = layout_model(&store, Some(alpha_id), &[alpha_id], &BTreeMap::new());
    assert!(model.selected_agent_rows
                 .iter()
                 .find(|row| row.id == alpha_id)
                 .unwrap()
                 .attached);
}

/// `agent-lifecycle`'s exit-driven removal is scoped by which agents own
/// a terminal process: a shell agent's exiting shell removes it, while an
/// ACP agent's adapter exiting does not (it has no PTY here at all).
#[test]
fn only_shell_agents_run_a_terminal_process() {
    assert!(runs_a_terminal_process("shell"));
    for agent_type in ["claude", "codex", "opencode", "gemini", "copilot"] {
        assert!(!runs_a_terminal_process(agent_type),
                "{agent_type} must not get a PTY under ACP-only launch");
    }
}

#[test]
fn state_label_matches_the_swift_reference_strings() {
    assert_eq!(state_label(knot_agents::AgentState::Idle), "Idle");
    assert_eq!(state_label(knot_agents::AgentState::Running), "Working");
    assert_eq!(state_label(knot_agents::AgentState::Input),
               "Awaiting input");
    assert_eq!(state_label(knot_agents::AgentState::Error), "Error");
}

#[test]
fn layout_model_carries_agent_state_into_rows() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.set_state(id, knot_agents::AgentState::Input);

    let model = layout_model(&store, None, &[], &BTreeMap::new());
    assert_eq!(model.selected_agent_rows[0].state,
               knot_agents::AgentState::Input);
}
