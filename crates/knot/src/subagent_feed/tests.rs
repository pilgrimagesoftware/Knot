//! The ACP feed, driven without an adapter subprocess.
//!
//! These build `SessionUpdate`s directly rather than decoding fixtures: the
//! wire shape is `knot-subagents`' contract and is tested there, against
//! committed fixtures. What is being tested here is the wiring - that a
//! recognized event reaches the registry, and that the two clearing signals
//! actually clear.

use std::sync::Arc;

use knot_acp::{SessionEndCause, SessionEvent, SessionUpdate, ToolCallContent};
use knot_subagents::registry::SubagentRegistry;
use parking_lot::Mutex;
use serde_json::json;
use uuid::Uuid;

use super::SubagentSink;

fn registry() -> Arc<Mutex<SubagentRegistry>> {
    Arc::new(Mutex::new(SubagentRegistry::new()))
}

fn delegation_start(id: &str) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallStart { tool_call_id: id.to_string(),
                                                        kind:         "think".to_string(),
                                                        title:        "Map the callers".to_string(),
                                                        status:       "pending".to_string(),
                                                        content:      Vec::new(),
                                                        raw_input:    Some(
        json!({ "subagent_type": "discovery",
                                   "description": "Map the callers" }),
    ),
                                                        meta:         Some(
        json!({ "claudeCode": { "toolName": "Task", "subagent": true } }),
    ), })
}

fn delegation_end(id: &str, status: &str, text: Option<&str>) -> SessionEvent {
    SessionEvent::Update(SessionUpdate::ToolCallUpdate {
        tool_call_id: id.to_string(),
        status:       Some(status.to_string()),
        title:        None,
        content:      text.map(|t| vec![ToolCallContent::Text(t.to_string())])
                          .unwrap_or_default(),
        // The completion carries the narrower meta the adapter really sends.
        raw_input:    None,
        meta:         Some(json!({ "claudeCode": { "toolName": "Task" } })),
    })
}

#[test]
fn a_type_with_no_recognizer_gets_no_sink() {
    let agent = Uuid::new_v4();

    assert!(SubagentSink::new(agent, "gemini", registry()).is_none());
    assert!(SubagentSink::new(agent, "shell", registry()).is_none());
    assert!(SubagentSink::new(agent, "claude", registry()).is_some());
}

#[test]
fn a_delegation_reaches_the_registry() {
    let (agent, registry) = (Uuid::new_v4(), registry());
    let sink = SubagentSink::new(agent, "claude", Arc::clone(&registry)).expect("claude has one");

    sink.observe(&delegation_start("tc1"));

    let held = registry.lock();
    let records = held.ordered(agent, std::time::Instant::now());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].task, "Map the callers");
    assert!(records[0].is_running());
}

#[test]
fn an_ordinary_tool_call_reaches_nothing() {
    let (agent, registry) = (Uuid::new_v4(), registry());
    let sink = SubagentSink::new(agent, "claude", Arc::clone(&registry)).expect("claude has one");

    sink.observe(&SessionEvent::Update(SessionUpdate::ToolCallStart {
        tool_call_id: "tc1".to_string(),
        kind:         "execute".to_string(),
        title:        "cargo test".to_string(),
        status:       "pending".to_string(),
        content:      Vec::new(),
        raw_input:    Some(json!({ "command": "cargo test" })),
        meta:         Some(json!({ "claudeCode": { "toolName": "Bash" } })),
    }));

    assert!(registry.lock().is_empty_for(agent));
}

#[test]
fn a_completion_moves_the_record_and_keeps_its_reason() {
    let (agent, registry) = (Uuid::new_v4(), registry());
    let sink = SubagentSink::new(agent, "claude", Arc::clone(&registry)).expect("claude has one");

    sink.observe(&delegation_start("tc1"));
    sink.observe(&delegation_end("tc1", "failed", Some("no such persona")));

    let held = registry.lock();
    let records = held.ordered(agent, std::time::Instant::now());
    assert_eq!(records[0].state.failure_reason(), Some("no such persona"));
    assert!(!records[0].is_running());
}

/// The spec's clearing signal on this feed.
#[test]
fn a_turn_ending_clears_the_records() {
    let (agent, registry) = (Uuid::new_v4(), registry());
    let sink = SubagentSink::new(agent, "claude", Arc::clone(&registry)).expect("claude has one");

    sink.observe(&delegation_start("tc1"));
    sink.observe(&SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason: "end_turn".to_string(), }));

    assert!(registry.lock().is_empty_for(agent));
}

/// An agent that dies mid-turn sends no turn-end, so the session ending has
/// to clear too - otherwise the records outlive the process they described.
#[test]
fn a_session_ending_clears_the_records() {
    let (agent, registry) = (Uuid::new_v4(), registry());
    let sink = SubagentSink::new(agent, "claude", Arc::clone(&registry)).expect("claude has one");

    sink.observe(&delegation_start("tc1"));
    sink.observe(&SessionEvent::Ended(SessionEndCause::BrokenPipe));

    assert!(registry.lock().is_empty_for(agent));
}

#[test]
fn one_agents_turn_ending_leaves_another_alone() {
    let (one, two, registry) = (Uuid::new_v4(), Uuid::new_v4(), registry());
    let sink_one = SubagentSink::new(one, "claude", Arc::clone(&registry)).expect("claude has one");
    let sink_two = SubagentSink::new(two, "claude", Arc::clone(&registry)).expect("claude has one");

    sink_one.observe(&delegation_start("tc1"));
    sink_two.observe(&delegation_start("tc1"));
    sink_one.observe(&SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason:
                                                                        "end_turn".to_string(), }));

    assert!(registry.lock().is_empty_for(one));
    assert!(!registry.lock().is_empty_for(two));
}

/// Two agents may each hold a subagent the adapter numbered the same, which
/// is why the registry keys on the agent before the subagent.
#[test]
fn identically_named_subagents_of_two_agents_stay_separate() {
    let (one, two, registry) = (Uuid::new_v4(), Uuid::new_v4(), registry());
    let sink_one = SubagentSink::new(one, "claude", Arc::clone(&registry)).expect("claude has one");
    let sink_two = SubagentSink::new(two, "claude", Arc::clone(&registry)).expect("claude has one");

    sink_one.observe(&delegation_start("tc1"));
    sink_two.observe(&delegation_start("tc1"));
    sink_one.observe(&delegation_end("tc1", "completed", None));

    let held = registry.lock();
    let now = std::time::Instant::now();
    assert!(!held.ordered(one, now)[0].is_running());
    assert!(held.ordered(two, now)[0].is_running(),
            "the other agent's was completed too");
}
