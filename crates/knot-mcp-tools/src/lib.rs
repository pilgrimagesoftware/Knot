//! Concrete MCP tool catalog exposed to agents.
//!
//! Implements `openspec/specs/mcp-tools/spec.md`.

mod agents;
mod args;
mod catalog;
mod consts;
mod error;
mod lookup;
mod messaging;
mod panels;
mod repos;
mod responses;

use std::collections::HashMap;
use std::sync::Arc;

use knot_activity::{EventSink, Tracker, TrackerConfig, tracking_for};
use knot_agents::{AgentState, AgentStore};
use knot_core::BenchAgent;
use knot_core::Settings;
use knot_discovery::RepoInfo;
use knot_mcp::{
    AgentHookHandler, HookRequest, HookStatus, claude_status, codex_turn_complete, extract_metadata,
};
use knot_messaging::{DeliveryNotifier, MessageStore};
use parking_lot::Mutex;
use tokio::sync::watch;
use uuid::Uuid;

pub use crate::error::{Result, ToolError};
use crate::lookup::state_string;

type AwaitingInputQueue = Arc<Mutex<Vec<(Uuid, Option<String>)>>>;

/// The concrete `ToolCatalog` for the thirteen tools in
/// `openspec/specs/mcp-tools/spec.md`. Holds every piece of shared state a
/// handler needs; each `call` locks only what that tool touches.
pub struct McpToolCatalog {
    agents:         Arc<Mutex<AgentStore>>,
    messages:       Arc<Mutex<MessageStore>>,
    notifier:       Arc<dyn DeliveryNotifier + Send + Sync>,
    repos:          watch::Receiver<Vec<RepoInfo>>,
    bench_agents:   Mutex<Vec<BenchAgent>>,
    settings:       Mutex<Option<Settings>>,
    trackers:       Mutex<HashMap<Uuid, Tracker>>,
    awaiting_input: Mutex<Option<AwaitingInputQueue>>,
}

impl McpToolCatalog {
    /// Takes the same `Arc<Mutex<AgentStore>>` the binary's shell renders, so
    /// MCP-driven mutations (`register-agent`, `set-status`, `create-agent`,
    /// hooks) land in the store the UI reads.
    pub fn new(agents: Arc<Mutex<AgentStore>>, repos: watch::Receiver<Vec<RepoInfo>>,
               notifier: Arc<dyn DeliveryNotifier + Send + Sync>)
               -> Self {
        Self { agents,
               messages: Arc::new(Mutex::new(MessageStore::new())),
               notifier,
               repos,
               bench_agents: Mutex::new(Vec::new()),
               settings: Mutex::new(None),
               trackers: Mutex::new(HashMap::new()),
               awaiting_input: Mutex::new(None) }
    }

    pub fn with_message_store(mut self, messages: Arc<Mutex<MessageStore>>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_awaiting_input_queue(self, queue: AwaitingInputQueue) -> Self {
        *self.awaiting_input.lock() = Some(queue);
        self
    }

    /// Refreshes the bench-agent templates `create-agent`'s `benchAgentId`
    /// resolves against. The catalog has no settings dependency of its own;
    /// the caller (`crates/knot`) pushes updates in whenever settings
    /// change.
    pub fn set_bench_agents(&self, bench_agents: Vec<BenchAgent>) {
        *self.bench_agents.lock() = bench_agents;
    }

    /// A clone of the current agent list, for `McpServer`'s
    /// `AgentsSnapshotFn` (the `GET /api/v1/agent/status` endpoint).
    pub fn agents_snapshot(&self) -> Vec<knot_agents::Agent> {
        self.agents.lock().agents().to_vec()
    }

    pub fn with_settings(self, settings: Settings) -> Self {
        *self.settings.lock() = Some(settings);
        self
    }

    fn persist_agent_state(&self) -> Result<()> {
        let agents = self.agents.lock();
        let mut settings = self.settings.lock();
        let Some(settings) = settings.as_mut()
        else {
            return Ok(());
        };
        settings.saved_agents = agents.saved_agents(settings.restore_conversation_on_launch);
        settings.saved_workspaces = agents.saved_workspaces();
        Ok(settings.persist()?)
    }

    fn tracker_for(&self, id: Uuid, agent_type: &str) -> bool {
        if !matches!(agent_type, "claude" | "codex") {
            return false;
        }
        let mut trackers = self.trackers.lock();
        if trackers.contains_key(&id) {
            return true;
        }
        if tokio::runtime::Handle::try_current().is_err() {
            return false;
        }

        let agents = Arc::clone(&self.agents);
        let awaiting_input = self.awaiting_input.lock().clone();
        let sink = EventSink { on_status: Some(Box::new(move |event| {
                                                   agents.lock().set_state(id, event.status);
                                               })),
                               on_awaiting_input: awaiting_input.map(|queue| {
                                                                    Box::new(move |message| {
                    {
                        let mut queue = queue.lock();
                        queue.push((id, message));
                    }
                }) as Box<dyn FnMut(Option<String>) + Send>
                                                                }),
                               ..Default::default() };
        // Hook-reported status is exclusively a Terminal-mode path - a
        // Panel-mode agent's status is driven by ACP session events
        // instead, never by this hook tracker.
        let tracker = Tracker::spawn(TrackerConfig { is_hook_based: true,
                                                     ..TrackerConfig::for_agent_type(agent_type) },
                                     tracking_for(agent_type, knot_core::ViewMode::Terminal),
                                     sink);
        trackers.insert(id, tracker);
        true
    }
}

impl AgentHookHandler for McpToolCatalog {
    fn register(&self, request: &HookRequest) -> Result<serde_json::Value, knot_mcp::HookError> {
        if request.agent != "claude" {
            return Err(knot_mcp::HookError::UnknownAgent(request.agent.clone()));
        }
        let id = request.agent_id()?;
        let mut agents = self.agents.lock();
        let Some(agent) = agents.agent(id)
        else {
            return Err(knot_mcp::HookError::InvalidPayload("Agent not found".to_string()));
        };
        let is_resuming = agent.resume_session_id.is_some() && !agent.fork_session;
        let is_fork = agent.fork_session;
        let source = request.source.as_deref().unwrap_or("startup");
        agents.set_registered(id, true);
        agents.update_metadata(id, extract_metadata("claude", &request.payload));
        if source == "resume" {
            if !is_fork && let Some(session_id) = request.session_id.clone() {
                agents.set_session_id(id, session_id);
            }
        }
        else {
            if !is_resuming && let Some(session_id) = request.session_id.clone() {
                agents.set_session_id(id, session_id);
            }
        }
        Ok(serde_json::json!({
            "success": true,
            "message": "Registered",
            "unreadMessageCount": 0,
            "knotMembers": agents.agents().iter().map(|agent| serde_json::json!({
                "id": agent.id.to_string(),
                "name": agent.name,
                "folder": agent.folder,
                "status": state_string(agent.state),
                "isRegistered": agent.is_registered,
            })).collect::<Vec<_>>(),
        }))
    }

    fn status(&self, request: &HookRequest) -> Result<serde_json::Value, knot_mcp::HookError> {
        let id = request.agent_id()?;
        if self.agents.lock().agent(id).is_none() {
            return Err(knot_mcp::HookError::InvalidPayload("Agent not found".to_string()));
        }
        self.agents
            .lock()
            .update_metadata(id, extract_metadata(&request.agent, &request.payload));
        let status = match request.agent.as_str() {
            "claude" => claude_status(
                request
                    .status
                    .as_deref()
                    .ok_or_else(|| knot_mcp::HookError::InvalidStatus("missing".to_string()))?,
            )?,
            "codex" => {
                let thread_id = codex_turn_complete(&request.payload)?;
                if let Some(thread_id) = thread_id {
                    self.agents.lock().set_session_id(id, thread_id);
                }
                HookStatus::Idle
            }
            agent => return Err(knot_mcp::HookError::UnknownAgent(agent.to_string())),
        };
        let state = match status {
            HookStatus::Working => AgentState::Running,
            HookStatus::Idle => AgentState::Idle,
            HookStatus::AwaitingInput => AgentState::Input,
        };
        let input_message =
            matches!(status, HookStatus::AwaitingInput).then(|| {
                                                           hook_input_message(&request.payload)
                                                       })
                                                       .flatten();
        if self.tracker_for(id, &request.agent) {
            self.trackers
                .lock()
                .get(&id)
                .expect("tracker inserted above")
                .apply_hook_status(state, input_message);
        }
        else {
            self.agents.lock().set_state(id, state);
        }
        Ok(serde_json::json!({"success": true}))
    }
}

fn hook_input_message(payload: &serde_json::Value) -> Option<String> {
    payload.get("message")
           .and_then(serde_json::Value::as_str)
           .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    // The catalogue and dispatch moved to `catalog.rs`; these exercise them
    // through the trait, so it has to be in scope here now.
    use knot_mcp::ToolCatalog;
    use knot_messaging::NoopNotifier;
    use tempfile::tempdir;

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

        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
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
                    .create("/tmp/a", knot_agents::CreateOptions::default());

        let result = cat.call(consts::REGISTER_AGENT,
                              serde_json::json!({"agentId": id.to_string()}))
                        .await;

        assert_eq!(result.is_error, None);
        assert!(cat.agents.lock().agent(id).unwrap().is_registered);
    }

    #[tokio::test]
    async fn successful_agent_mutation_persists_durable_state() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let cat = catalog().with_settings(knot_core::Settings::with_store_path(&path));
        let id = cat.agents
                    .lock()
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
                    .create("/tmp/a", knot_agents::CreateOptions::default());
        let request: HookRequest =
            serde_json::from_value(serde_json::json!({
                                       "agent_id": id,
                                       "session_id": "session-1",
                                       "payload": {"cwd": "/tmp/a", "model": "claude-sonnet"}
                                   })).unwrap();

        cat.register(&request).unwrap();
        let agent = cat.agents.lock().agent(id).unwrap().clone();
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
                    .create("/tmp/a", knot_agents::CreateOptions::default());
        let request: HookRequest = serde_json::from_value(serde_json::json!({
            "agent_id": id,
            "agent": "codex",
            "payload": {"type": "agent-turn-complete", "thread-id": "thread-1"}
        }))
        .unwrap();

        cat.status(&request).unwrap();
        tokio::task::yield_now().await;
        let agent = cat.agents.lock().agent(id).unwrap().clone();
        assert_eq!(agent.state, AgentState::Idle);
        assert_eq!(agent.session_id.as_deref(), Some("thread-1"));
    }

    #[tokio::test]
    async fn claude_hook_status_updates_state_through_tracker() {
        let cat = catalog();
        let id = cat.agents
                    .lock()
                    .create("/tmp/a", knot_agents::CreateOptions::default());
        let request: HookRequest = serde_json::from_value(serde_json::json!({
                                                              "agent_id": id,
                                                              "agent": "claude",
                                                              "status": "input"
                                                          })).unwrap();

        cat.status(&request).unwrap();
        tokio::task::yield_now().await;
        assert_eq!(cat.agents.lock().agent(id).unwrap().state,
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
                       .create("/tmp/one", knot_agents::CreateOptions::default());

        assert_eq!(a.agents_snapshot().len(), 1);
        assert_eq!(b.agents_snapshot().len(), 1);

        shared.lock().set_status_text(id, "planning".to_string());
        assert_eq!(a.agents_snapshot()[0].status_text, "planning");
        assert_eq!(b.agents_snapshot()[0].status_text, "planning");
    }

    #[test]
    fn hook_input_message_extracts_optional_payload_message() {
        assert_eq!(hook_input_message(&serde_json::json!({"message": "Question?"})),
                   Some("Question?".to_string()));
        assert_eq!(hook_input_message(&serde_json::json!({"message": 42})),
                   None);
        assert_eq!(hook_input_message(&serde_json::json!({})), None);
    }
}
