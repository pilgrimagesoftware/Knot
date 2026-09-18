use std::sync::{Arc, Mutex};

use knot_agents::{AgentState, AgentStore};
use knot_mcp::{AgentHookHandler, HookRequest, ToolCatalog};
use knot_messaging::NoopNotifier;
use tempfile::tempdir;
use tokio::sync::watch;

use super::*;

fn catalog() -> McpToolCatalog {
    let (_tx, rx) = watch::channel(Vec::new());
    McpToolCatalog::new(Arc::new(Mutex::new(AgentStore::new())),
                        rx,
                        Arc::new(NoopNotifier))
}

#[test]
fn lists_exactly_the_thirteen_tools_with_object_schemas() {
    let defs = catalog().list();
    assert_eq!(defs.len(), 13);
    for def in &defs {
        assert_eq!(def.input_schema.schema_type, "object");
    }
    let names: Vec<&str> = defs.iter().map(|def| def.name.as_str()).collect();
    for expected in ["register-agent",
                     "list-agents",
                     "send-message",
                     "check-messages",
                     "broadcast-message",
                     "list-repos",
                     "list-worktrees",
                     "create-agent",
                     "close-agent",
                     "create-worktree",
                     "set-status",
                     "display-markdown",
                     "view-mermaid"]
    {
        assert!(names.contains(&expected), "missing tool: {expected}");
    }
}

#[tokio::test]
async fn unknown_tool_name_is_a_tool_error() {
    let result = catalog().call("does-not-exist", serde_json::json!({}))
                          .await;
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("does-not-exist"));
}

#[tokio::test]
async fn dispatches_register_agent_by_name() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .unwrap()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let result = cat.call(consts::REGISTER_AGENT,
                          serde_json::json!({"agentId": id.to_string()}))
                    .await;
    assert_eq!(result.is_error, None);
    assert!(cat.agents.lock().unwrap().agent(id).unwrap().is_registered);
}

#[tokio::test]
async fn successful_agent_mutation_persists_durable_state() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let cat = catalog().with_settings(knot_core::Settings::with_store_path(&path));
    let id = cat.agents
                .lock()
                .unwrap()
                .create("/tmp/persisted", knot_agents::CreateOptions::default());
    let result = cat.call(consts::REGISTER_AGENT,
                          serde_json::json!({"agentId": id.to_string()}))
                    .await;
    assert!(result.is_error.is_none());
    let settings = knot_core::Settings::load_from(path).unwrap();
    assert_eq!(settings.saved_agents.len(), 1);
    assert_eq!(settings.saved_agents[0].id, id);
    assert_eq!(settings.saved_workspaces.len(), 1);
    assert_eq!(settings.saved_workspaces[0].agent_ids, vec![id]);
}

#[test]
fn hook_registration_updates_session_and_metadata() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .unwrap()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let request: HookRequest = serde_json::from_value(serde_json::json!({
        "agent_id": id, "session_id": "session-1", "payload": {"cwd": "/tmp/a", "model": "claude-sonnet"}
    }))
    .unwrap();
    cat.register(&request).unwrap();
    let agent = cat.agents.lock().unwrap().agent(id).unwrap().clone();
    assert!(agent.is_registered);
    assert_eq!(agent.session_id.as_deref(), Some("session-1"));
    assert_eq!(agent.metadata.get("model"),
               Some(&"claude-sonnet".to_string()));
}

#[tokio::test]
async fn hook_status_updates_state_through_tracker_and_codex_session() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .unwrap()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let request: HookRequest = serde_json::from_value(serde_json::json!({
        "agent_id": id, "agent": "codex", "payload": {"type": "agent-turn-complete", "thread-id": "thread-1"}
    }))
    .unwrap();
    cat.status(&request).unwrap();
    tokio::task::yield_now().await;
    let agent = cat.agents.lock().unwrap().agent(id).unwrap().clone();
    assert_eq!(agent.state, AgentState::Idle);
    assert_eq!(agent.session_id.as_deref(), Some("thread-1"));
}

#[tokio::test]
async fn claude_hook_status_updates_state_through_tracker() {
    let cat = catalog();
    let id = cat.agents
                .lock()
                .unwrap()
                .create("/tmp/a", knot_agents::CreateOptions::default());
    let request: HookRequest =
        serde_json::from_value(serde_json::json!({
                                   "agent_id": id, "agent": "claude", "status": "input"
                               })).unwrap();
    cat.status(&request).unwrap();
    tokio::task::yield_now().await;
    assert_eq!(cat.agents.lock().unwrap().agent(id).unwrap().state,
               AgentState::Input);
}

#[test]
fn catalogs_sharing_a_store_observe_each_others_mutations() {
    let (_tx, rx_a) = watch::channel(Vec::new());
    let (_tx, rx_b) = watch::channel(Vec::new());
    let shared = Arc::new(Mutex::new(AgentStore::new()));
    let a = McpToolCatalog::new(Arc::clone(&shared), rx_a, Arc::new(NoopNotifier));
    let b = McpToolCatalog::new(Arc::clone(&shared), rx_b, Arc::new(NoopNotifier));
    let id = shared.lock()
                   .unwrap()
                   .create("/tmp/one", knot_agents::CreateOptions::default());
    assert_eq!(a.agents_snapshot().len(), 1);
    assert_eq!(b.agents_snapshot().len(), 1);
    shared.lock()
          .unwrap()
          .set_status_text(id, "planning".to_string());
    assert_eq!(a.agents_snapshot()[0].status_text, "planning");
    assert_eq!(b.agents_snapshot()[0].status_text, "planning");
}

#[test]
fn hook_input_message_extracts_optional_payload_message() {
    assert_eq!(hooks::hook_input_message(&serde_json::json!({"message": "Question?"})),
               Some("Question?".to_string()));
    assert_eq!(hooks::hook_input_message(&serde_json::json!({"message": 42})),
               None);
    assert_eq!(hooks::hook_input_message(&serde_json::json!({})), None);
}
