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
mod tasks;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use knot_activity::{EventSink, Tracker, TrackerConfig, tracking_for};
use knot_agents::{AgentState, AgentStore};
use knot_core::BenchAgent;
use knot_core::SharedSettings;
use knot_discovery::RepoInfo;
use knot_mcp::{
    AgentHookHandler, HookRequest, HookStatus, claude_status, codex_turn_complete, extract_metadata,
};
use knot_messaging::{DeliveryNotifier, MessageStore};
use knot_subagents::recognize::hooks::{
    is_subagent_hook, recognize_hook, subagent_payload_is_complete,
};
use knot_subagents::registry::SubagentRegistry;
use parking_lot::Mutex;
use tokio::sync::watch;
use uuid::Uuid;

pub use crate::error::{Result, ToolError};
use crate::lookup::state_string;
pub use crate::tasks::{GraphStore, plan_mermaid, plan_tasks};

type AwaitingInputQueue = Arc<Mutex<Vec<(Uuid, Option<String>)>>>;

/// Recipients a direct `send-message` delivered to while they were not
/// activated, for the app to start. Held as an `Option` because the catalog
/// is constructed in tests and benches with no window to drain it; see
/// `messaging::send_message`.
pub type ActivationQueue = Arc<Mutex<Vec<Uuid>>>;

/// The shared subagent registry, as the catalog holds it.
pub type SubagentRegistryHandle = Arc<Mutex<SubagentRegistry>>;

/// The concrete `ToolCatalog` for the tools in
/// `openspec/specs/mcp-tools/spec.md`. Holds every piece of shared state a
/// handler needs; each `call` locks only what that tool touches.
pub struct McpToolCatalog {
    agents:         Arc<Mutex<AgentStore>>,
    messages:       Arc<Mutex<MessageStore>>,
    notifier:       Arc<dyn DeliveryNotifier + Send + Sync>,
    repos:          watch::Receiver<Vec<RepoInfo>>,
    bench_agents:   Mutex<Vec<BenchAgent>>,
    settings:       Mutex<Option<SharedSettings>>,
    trackers:       Mutex<HashMap<Uuid, Tracker>>,
    awaiting_input: Mutex<Option<AwaitingInputQueue>>,
    activation:     Mutex<Option<ActivationQueue>>,
    /// Committed task plans, one per owning agent. Runtime state, like the
    /// message queue: not persisted, gone with the process.
    graphs:         Mutex<tasks::GraphStore>,
    /// Where hook-reported subagents are recorded, shared with the windows
    /// that draw them and with the ACP feed that writes to it too.
    ///
    /// `None` in a catalog built without one - the tests, and any embedding
    /// that has no UI - in which case subagent events validate and are
    /// acknowledged but recorded nowhere. That is deliberate: the route's
    /// contract with the poster is about the payload, not about whether
    /// anything downstream happens to be listening.
    subagents:      Option<SubagentRegistryHandle>,
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
               awaiting_input: Mutex::new(None),
               activation: Mutex::new(None),
               graphs: Mutex::new(tasks::GraphStore::new()),
               subagents: None }
    }

    pub fn with_message_store(mut self, messages: Arc<Mutex<MessageStore>>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_awaiting_input_queue(self, queue: AwaitingInputQueue) -> Self {
        *self.awaiting_input.lock() = Some(queue);
        self
    }

    pub fn with_activation_queue(self, queue: ActivationQueue) -> Self {
        *self.activation.lock() = Some(queue);
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

    /// The settings surface this catalog persists the roster through.
    ///
    /// Takes the shared handle rather than an owned `Settings`. The catalog
    /// lives on the MCP server's own thread for the whole session, so a
    /// snapshot taken when the server started would still be read hours
    /// later - `restore_conversation_on_launch` below is exactly that read,
    /// and it decides what a persisted agent carries. Issue #238 in a
    /// background service.
    pub fn with_settings(self, settings: SharedSettings) -> Self {
        *self.settings.lock() = Some(settings);
        self
    }

    fn persist_agent_state(&self) -> Result<()> {
        let agents = self.agents.lock();
        let settings = self.settings.lock();
        let Some(settings) = settings.as_ref()
        else {
            return Ok(());
        };
        let installed = settings.write(|settings| {
                                    settings.saved_agents =
                                agents.saved_agents(settings.restore_conversation_on_launch);
                                    settings.saved_workspaces = agents.saved_workspaces();
                                });
        Ok(installed.persist_roster()?)
    }

    fn tracker_for(&self, id: Uuid, agent_type: &str) -> bool {
        // Only the types whose hooks report progress get a tracker; for
        // the rest it would arm timers nothing ever feeds.
        if !knot_core::agent_type::has_hook_activity(agent_type) {
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
        // Before the status vocabulary is consulted, and returning early: a
        // subagent event is keyed on `hook`, carries no `status`, and must
        // not move the agent's activity state as a side effect of being
        // reported. Falling through would reject it as a missing status.
        if let Some(hook) = request.hook.as_deref()
           && is_subagent_hook(hook)
        {
            return self.subagent_event(id, hook, &request.payload);
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
        // Going idle is this feed's turn-end: the agent has stopped working,
        // so whatever it delegated during the turn is over. The ACP feed has
        // `TurnEnd` for the same job. Without this, a terminal agent's
        // subagents would accumulate across every turn of a session.
        if matches!(state, AgentState::Idle)
           && let Some(subagents) = &self.subagents
        {
            subagents.lock().clear(id);
        }
        Ok(serde_json::json!({"success": true}))
    }
}

impl McpToolCatalog {
    /// Shares the window's subagent registry, so hook-reported delegations
    /// land in the same place the ACP feed writes to.
    #[must_use]
    pub fn with_subagents(mut self, subagents: SubagentRegistryHandle) -> Self {
        self.subagents = Some(subagents);
        self
    }

    /// Records one subagent lifecycle event.
    ///
    /// A malformed payload is a 400, per `agent-hooks`. An event naming a
    /// subagent with no recorded dispatch is *not*: Knot may have started
    /// after the dispatch, and answering an error there would make the
    /// agent's hook report a failure for something it did correctly. The
    /// registry drops it silently.
    fn subagent_event(&self, id: Uuid, hook: &str, payload: &serde_json::Value)
                      -> Result<serde_json::Value, knot_mcp::HookError> {
        if !subagent_payload_is_complete(hook, payload) {
            return Err(knot_mcp::HookError::InvalidPayload(
                "Subagent event is missing a required field".to_string(),
            ));
        }

        if let Some(subagents) = &self.subagents
           && let Some(event) = recognize_hook(hook, payload)
        {
            subagents.lock().apply(id, event, Instant::now());
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
    fn lists_exactly_the_catalogued_tools_with_object_schemas() {
        let defs = catalog().list();
        assert_eq!(defs.len(), 18);
        for def in &defs {
            assert_eq!(def.input_schema.schema_type, "object");
        }

        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        for expected in ["register-agent",
                         "list-agents",
                         "describe-agents",
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
                         "view-mermaid",
                         "plan-tasks",
                         "dispatch-task",
                         "complete-task",
                         "task-status"]
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
        let cat = catalog().with_settings(knot_core::SharedSettings::new(knot_core::Settings::with_store_root(dir.path())));
        let id = cat.agents
                    .lock()
                    .create("/tmp/persisted", knot_agents::CreateOptions::default());

        let result = cat.call(consts::REGISTER_AGENT,
                              serde_json::json!({"agentId": id.to_string()}))
                        .await;

        assert!(result.is_error.is_none());
        let settings = knot_core::Settings::load_from_root(dir.path()).unwrap();
        assert_eq!(settings.saved_agents.len(), 1);
        assert_eq!(settings.saved_agents[0].id, id);
        assert_eq!(settings.saved_workspaces.len(), 1);
        assert_eq!(settings.saved_workspaces[0].agent_ids, vec![id]);
    }

    /// A subagent event is keyed on `hook`, carries no `status`, and must not
    /// move the agent's activity state as a side effect of being reported.
    #[test]
    fn a_subagent_dispatch_is_recorded_without_touching_the_agents_state() {
        let subagents: SubagentRegistryHandle = Arc::default();
        let cat = catalog().with_subagents(Arc::clone(&subagents));
        let id = cat.agents
                    .lock()
                    .create("/tmp/a", knot_agents::CreateOptions::default());
        cat.agents.lock().set_state(id, AgentState::Running);

        cat.status(&subagent_hook(id,
                                  "SubagentStart",
                                  serde_json::json!({
                                      "subagent_id": "sub-1",
                                      "subagent_type": "discovery",
                                      "task": "Map the callers"
                                  })))
           .expect("a complete dispatch is accepted");

        assert_eq!(cat.agents.lock().agent(id).unwrap().state,
                   AgentState::Running,
                   "reporting a subagent moved the agent's own status");
        let held = subagents.lock();
        let records = held.ordered(id, std::time::Instant::now());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].task, "Map the callers");
    }

    #[test]
    fn a_subagent_completion_moves_its_record() {
        let subagents: SubagentRegistryHandle = Arc::default();
        let cat = catalog().with_subagents(Arc::clone(&subagents));
        let id = cat.agents
                    .lock()
                    .create("/tmp/a", knot_agents::CreateOptions::default());

        cat.status(&subagent_hook(id,
                                  "SubagentStart",
                                  serde_json::json!({ "subagent_id": "sub-1",
                                                      "task": "Map the callers" })))
           .unwrap();
        cat.status(&subagent_hook(id,
                                  "SubagentStop",
                                  serde_json::json!({ "subagent_id": "sub-1",
                                                      "outcome": "failed",
                                                      "reason": "no such persona" })))
           .unwrap();

        let held = subagents.lock();
        let records = held.ordered(id, std::time::Instant::now());
        assert_eq!(records[0].state.failure_reason(), Some("no such persona"));
    }

    /// The payload is the poster's contract, so a malformed one is an error it
    /// can act on rather than a silent no-op.
    #[test]
    fn a_subagent_event_missing_a_required_field_is_rejected() {
        let cat = catalog().with_subagents(Arc::default());
        let id = cat.agents
                    .lock()
                    .create("/tmp/a", knot_agents::CreateOptions::default());

        let no_id = subagent_hook(id, "SubagentStart", serde_json::json!({ "task": "go" }));
        let no_task = subagent_hook(id,
                                    "SubagentStart",
                                    serde_json::json!({ "subagent_id": "s" }));
        let no_outcome = subagent_hook(id,
                                       "SubagentStop",
                                       serde_json::json!({ "subagent_id": "s" }));

        assert!(cat.status(&no_id).is_err());
        assert!(cat.status(&no_task).is_err());
        assert!(cat.status(&no_outcome).is_err());
    }

    /// Knot may have started after the dispatch. Answering an error there would
    /// make the agent's hook report a failure for something it did correctly.
    #[test]
    fn a_completion_for_an_unknown_subagent_succeeds_and_records_nothing() {
        let subagents: SubagentRegistryHandle = Arc::default();
        let cat = catalog().with_subagents(Arc::clone(&subagents));
        let id = cat.agents
                    .lock()
                    .create("/tmp/a", knot_agents::CreateOptions::default());

        let request = subagent_hook(id,
                                    "SubagentStop",
                                    serde_json::json!({ "subagent_id": "never-seen",
                                                        "outcome": "succeeded" }));

        assert!(cat.status(&request).is_ok());
        assert!(subagents.lock().is_empty_for(id));
    }

    /// The subagent branch returns early, so a mistake there would silently
    /// stop every ordinary activity update.
    #[test]
    fn an_ordinary_status_hook_still_moves_the_agent() {
        let cat = catalog().with_subagents(Arc::default());
        let id = cat.agents
                    .lock()
                    .create("/tmp/a", knot_agents::CreateOptions::default());

        let request: HookRequest = serde_json::from_value(serde_json::json!({
                                                              "agent_id": id,
                                                              "hook": "Stop",
                                                              "status": "idle",
                                                              "payload": {}
                                                          })).unwrap();

        cat.status(&request)
           .expect("an ordinary status post is accepted");
        assert_eq!(cat.agents.lock().agent(id).unwrap().state, AgentState::Idle);
    }

    fn subagent_hook(agent: Uuid, hook: &str, payload: serde_json::Value) -> HookRequest {
        serde_json::from_value(serde_json::json!({
                                   "agent_id": agent,
                                   "hook": hook,
                                   "payload": payload
                               })).unwrap()
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
