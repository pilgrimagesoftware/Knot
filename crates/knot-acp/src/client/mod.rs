//! ACP semantics (`initialize`, session lifecycle, updates, permissions)
//! built on top of the generic [`Transport`].

use std::sync::Arc;
use std::sync::Mutex;

use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::error::{AcpError, Result};
use crate::protocol::{
    AgentCapabilities, ConfigOption, InitializeParams, InitializeResult, JsonRpcErrorPayload,
    MCP_SERVER_NAME, PROTOCOL_VERSION, PermissionDecision, PermissionOption, PermissionRequest,
    SessionUpdate,
};
use crate::transport::{Transport, TransportEvent};

mod events;
pub use events::{NewSession, SessionEvent};

/// Cheap to clone - `transport` and `permission_pending` are both `Arc`,
/// so every clone dispatches through the same underlying connection. Lets
/// a caller holding one behind a `Mutex` extract an owned handle to
/// `.await` on without keeping the lock held across the await point.
#[derive(Clone)]
pub struct AcpClient {
    transport: Arc<Transport>,
    capabilities: AgentCapabilities,
    /// Session Config Options declared on `initialize`, if any - seeds a
    /// new session's config options before `session/new`'s own (possibly
    /// richer, per-session) list arrives.
    init_config_options: Vec<ConfigOption>,
    permission_pending:
        Arc<Mutex<std::collections::HashMap<String, oneshot::Sender<PermissionDecision>>>>,
    /// The same channel the transport loop feeds. `session_prompt` needs
    /// it because ACP ends a turn by *responding* to `session/prompt`
    /// with a stop reason rather than sending a `session/update`, so the
    /// turn-end event has to be synthesized from that response.
    events: mpsc::UnboundedSender<SessionEvent>,
}

impl AcpClient {
    /// Spawns `command`, negotiates the protocol version, and returns the
    /// client plus the ordered session-event stream. Fails closed (per
    /// `acp-client`'s capability-negotiation requirement) if the agent
    /// reports an unsupported protocol version.
    pub async fn connect(
        command: Command,
    ) -> Result<(Self, mpsc::UnboundedReceiver<SessionEvent>)> {
        let (transport, mut transport_events) = Transport::spawn(command)?;

        let init_params = InitializeParams {
            protocol_version: PROTOCOL_VERSION,
        };
        let raw = transport
            .request(
                "initialize",
                Some(serde_json::to_value(init_params).expect("serializable")),
            )
            .await?;
        let init_result: InitializeResult =
            serde_json::from_value(raw).map_err(|error| AcpError::Rpc {
                code: -32600,
                message: format!("malformed initialize response: {error}"),
            })?;
        if init_result.protocol_version != PROTOCOL_VERSION {
            return Err(AcpError::UnsupportedProtocolVersion {
                client: PROTOCOL_VERSION,
                agent: init_result.protocol_version,
            });
        }

        let permission_pending: Arc<
            Mutex<std::collections::HashMap<String, oneshot::Sender<PermissionDecision>>>,
        > = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let events_for_client = events_tx.clone();

        let transport_for_loop = Arc::clone(&transport);
        let pending_for_loop = Arc::clone(&permission_pending);
        tokio::spawn(async move {
            while let Some(event) = transport_events.recv().await {
                match event {
                    TransportEvent::Notification { method, params }
                        if method == "session/update" =>
                    {
                        let update = SessionUpdate::from_params(params.unwrap_or(Value::Null));
                        let _ = events_tx.send(SessionEvent::Update(update));
                    }
                    TransportEvent::Notification { .. } => {}
                    TransportEvent::Request { id, method, params }
                        if method == "session/request_permission" =>
                    {
                        let params = params.unwrap_or(Value::Null);
                        // The spec nests the tool call:
                        // `params.toolCall.toolCallId`
                        // (<https://agentclientprotocol.com/protocol/tool-calls>,
                        // "Requesting Permission"). The flat spelling is
                        // kept as a fallback for adapters that send it.
                        let tool_call_id = params
                            .get("toolCall")
                            .and_then(|call| call.get("toolCallId"))
                            .or_else(|| params.get("toolCallId"))
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let tool_call_title = params
                            .get("toolCall")
                            .and_then(|call| call.get("title"))
                            .and_then(Value::as_str)
                            .map(str::to_owned);
                        let options: Vec<PermissionOption> = serde_json::from_value(
                            params
                                .get("options")
                                .cloned()
                                .unwrap_or(Value::Array(vec![])),
                        )
                        .unwrap_or_default();
                        let rpc_key = id.to_string();
                        let (decision_tx, decision_rx) = oneshot::channel();
                        pending_for_loop
                            .lock()
                            .expect("permission mutex poisoned")
                            .insert(rpc_key.clone(), decision_tx);
                        let _ =
                            events_tx.send(SessionEvent::PermissionRequest(PermissionRequest {
                                rpc_id: id.clone(),
                                tool_call_id,
                                tool_call_title,
                                options: options.clone(),
                            }));
                        let transport = Arc::clone(&transport_for_loop);
                        let options_for_answer = options;
                        tokio::spawn(async move {
                            let decision = decision_rx.await.unwrap_or(PermissionDecision::Deny);
                            let outcome = permission_result(decision, &options_for_answer);
                            let _ = transport.respond(id, outcome).await;
                        });
                    }
                    TransportEvent::Request { id, .. } => {
                        // Unrecognized server-to-client request: decline
                        // rather than leaving it (and the agent) hanging.
                        let _ = transport_for_loop
                            .respond(
                                id,
                                Err(JsonRpcErrorPayload {
                                    code: -32601,
                                    message: "method not supported".to_owned(),
                                    data: None,
                                }),
                            )
                            .await;
                    }
                    TransportEvent::Ended(cause) => {
                        // Per the "session closed while a permission request
                        // is pending" scenario: any still-pending permission
                        // decisions resolve to Deny rather than hanging.
                        let pending: Vec<_> = pending_for_loop
                            .lock()
                            .expect("permission mutex poisoned")
                            .drain()
                            .collect();
                        for (_, sender) in pending {
                            let _ = sender.send(PermissionDecision::Deny);
                        }
                        let _ = events_tx.send(SessionEvent::Ended(cause));
                        break;
                    }
                }
            }
        });

        Ok((
            Self {
                transport,
                capabilities: init_result.capabilities,
                init_config_options: init_result.config_options,
                permission_pending,
                events: events_for_client,
            },
            events_rx,
        ))
    }

    pub fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    pub async fn session_new(&self, cwd: &str, mcp_url: Option<&str>) -> Result<NewSession> {
        let raw = self
            .transport
            .request(
                "session/new",
                Some(json!({ "cwd": cwd,
                                          "mcpServers": self.mcp_servers(mcp_url) })),
            )
            .await?;
        let session_id = session_id_from(&raw)?;
        let config_options = config_options_from(&raw, &self.init_config_options);
        Ok(NewSession {
            session_id,
            config_options,
        })
    }

    /// Resumes a prior session. Returns a typed "not supported" error
    /// without sending the request when the agent's capabilities don't
    /// advertise `session/load` support.
    pub async fn session_load(
        &self, session_id: &str, cwd: &str, mcp_url: Option<&str>,
    ) -> Result<NewSession> {
        if !self.capabilities.supports_resume {
            return Err(AcpError::ResumeNotSupported);
        }
        let raw = self
            .transport
            .request(
                "session/load",
                Some(json!({ "sessionId": session_id, "cwd": cwd,
                                          "mcpServers": self.mcp_servers(mcp_url) })),
            )
            .await?;
        let session_id = session_id_from(&raw)?;
        let config_options = config_options_from(&raw, &self.init_config_options);
        Ok(NewSession {
            session_id,
            config_options,
        })
    }

    /// The `mcpServers` array for a `session/new`/`session/load` request:
    /// one entry naming Knot's own HTTP MCP server, in the ACP spec's
    /// `McpServerHttp` shape (`type`/`name`/`url`/`headers`, `headers`
    /// required even when empty - https://agentclientprotocol.com/protocol/v1/schema),
    /// when `mcp_url` is given AND the agent declared `mcpCapabilities.http`
    /// on `initialize` (sending an `http`-type entry to an agent that
    /// hasn't declared support for it is a protocol violation - confirmed
    /// live: opencode/gemini reject it with `Invalid params`/`Internal
    /// error`). Empty otherwise. `mcpServers` itself is still required by
    /// at least the Gemini CLI adapter even when empty.
    fn mcp_servers(&self, mcp_url: Option<&str>) -> Value {
        match mcp_url {
            Some(url) if self.capabilities.mcp_capabilities.http => {
                json!([{ "type": "http", "name": MCP_SERVER_NAME, "url": url,
                         "headers": [] }])
            }
            _ => json!([]),
        }
    }

    /// Applies one Session Config Option selection (mode, model, effort,
    /// ...) and returns the agent's updated full list, per the stabilized
    /// Session Config Options `session/set_config_option` response shape.
    pub async fn session_set_config_option(
        &self, session_id: &str, config_id: &str, value: &str,
    ) -> Result<Vec<ConfigOption>> {
        let raw = self
            .transport
            .request(
                "session/set_config_option",
                Some(json!({ "sessionId": session_id, "configId": config_id,
                                          "type": "id", "value": value })),
            )
            .await?;
        Ok(raw
            .get("configOptions")
            .cloned()
            .map(serde_json::from_value)
            .and_then(std::result::Result::ok)
            .unwrap_or_default())
    }

    /// Sends one prompt and waits for the turn to finish.
    ///
    /// ACP ends a prompt turn by responding to *this request* with a
    /// `stopReason` - there is no `turn_end` session update
    /// (<https://agentclientprotocol.com/protocol/prompt-turn>, "Check for
    /// Completion"). So the turn-end event callers rely on is synthesized
    /// from the response here. It is emitted even when the request fails,
    /// because a caller that gates input on "a turn is in flight" would
    /// otherwise stay blocked forever on a failed prompt.
    pub async fn session_prompt(&self, session_id: &str, text: &str) -> Result<()> {
        let result = self.transport
                         .request("session/prompt", Some(json!({ "sessionId": session_id, "prompt": [{ "type": "text", "text": text }] })))
                         .await;
        let stop_reason = match &result {
            Ok(raw) => raw
                .get("stopReason")
                .and_then(Value::as_str)
                .unwrap_or("end_turn")
                .to_owned(),
            Err(_) => "error".to_owned(),
        };
        let _ = self
            .events
            .send(SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }));
        result.map(|_| ())
    }

    pub async fn session_cancel(&self, session_id: &str) -> Result<()> {
        self.transport
            .notify("session/cancel", Some(json!({ "sessionId": session_id })))
            .await
    }

    /// Answers a pending `session/request_permission` request surfaced via
    /// [`SessionEvent::PermissionRequest`].
    pub fn answer_permission(&self, request: &PermissionRequest, decision: PermissionDecision) {
        let key = request.rpc_id.to_string();
        if let Some(sender) = self
            .permission_pending
            .lock()
            .expect("permission mutex poisoned")
            .remove(&key)
        {
            let _ = sender.send(decision);
        }
    }

    /// Closes the session by terminating the adapter subprocess. Any
    /// permission request still pending resolves to Deny (see the
    /// `Ended` handling in [`AcpClient::connect`]'s background loop)
    /// rather than being left unanswered.
    pub async fn close(&self) {
        self.transport.close().await;
    }
}

fn session_id_from(raw: &Value) -> Result<String> {
    raw.get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AcpError::Rpc {
            code: -32600,
            message: "session response missing sessionId".to_owned(),
        })
}

/// A `session/new`/`session/load` response's own `configOptions`, falling
/// back to whatever `initialize` already declared when the per-session
/// response omits them (the stabilized spec allows declaring them in
/// either place).
fn config_options_from(raw: &Value, init_config_options: &[ConfigOption]) -> Vec<ConfigOption> {
    raw.get("configOptions")
        .cloned()
        .map(serde_json::from_value)
        .and_then(std::result::Result::ok)
        .unwrap_or_else(|| init_config_options.to_vec())
}

fn permission_result(
    decision: PermissionDecision, options: &[PermissionOption],
) -> std::result::Result<Value, JsonRpcErrorPayload> {
    let outcome = match decision {
        PermissionDecision::Allow => options
            .first()
            .map(|option| option.option_id.clone())
            .unwrap_or_else(|| "allow".to_owned()),
        PermissionDecision::Deny => options
            .iter()
            .find(|option| {
                option.option_id.to_lowercase().contains("deny")
                    || option.name.to_lowercase().contains("deny")
            })
            .map(|option| option.option_id.clone())
            .unwrap_or_else(|| "deny".to_owned()),
    };
    Ok(json!({ "outcome": { "outcome": "selected", "optionId": outcome } }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SessionEndCause;

    /// A fake agent handling `initialize` (with a configurable protocol
    /// version and resume capability) and `session/new`/`session/load`.
    fn fake_agent(protocol_version: u32, supports_resume: bool) -> Command {
        let mut command = Command::new("sh");
        let script = format!(
            r#"while IFS= read -r line; do
              id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
              case "$method" in
                initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{protocol_version},\"agentCapabilities\":{{\"loadSession\":{supports_resume}}}}}}}" ;;
                session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}" ;;
                *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
              esac
            done"#
        );
        command.arg("-c").arg(script);
        command
    }

    /// A fake agent whose `session/prompt` answers with a stop reason and
    /// sends no `turn_end` notification - which is how ACP actually ends a
    /// turn.
    fn prompting_agent() -> Command {
        let mut command = Command::new("sh");
        let script = format!(
            r#"while IFS= read -r line; do
              id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
              case "$method" in
                initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{}}}}}}" ;;
                session/prompt) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"stopReason\":\"end_turn\"}}}}" ;;
                *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
              esac
            done"#
        );
        command.arg("-c").arg(script);
        command
    }

    /// The turn-end event has to be synthesized from the `session/prompt`
    /// response, because ACP has no `turn_end` session update. Without it a
    /// caller that gates input on "a turn is in flight" - the panel's Send
    /// button and Enter key both do - stays blocked forever after the very
    /// first prompt.
    #[tokio::test]
    async fn a_finished_prompt_emits_a_turn_end_with_the_responses_stop_reason() {
        let (client, mut events) = AcpClient::connect(prompting_agent())
            .await
            .expect("connect");

        client
            .session_prompt("sess-1", "hello")
            .await
            .expect("prompt");

        let event = events.recv().await.expect("a turn-end event");
        assert!(
            matches!(&event,
                         SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason })
                         if stop_reason == "end_turn"),
            "expected a turn end, got {event:?}"
        );
    }

    /// A prompt that fails must still end the turn, or the same gate wedges
    /// the panel permanently on one bad request.
    #[tokio::test]
    async fn a_failed_prompt_still_ends_the_turn() {
        let (client, mut events) = AcpClient::connect(fake_agent(PROTOCOL_VERSION, true))
            .await
            .expect("connect");
        drop(client.close());

        let _ = client.session_prompt("sess-1", "hello").await;

        let event = events.recv().await.expect("a turn-end event");
        assert!(
            matches!(&event, SessionEvent::Update(SessionUpdate::TurnEnd { .. })),
            "expected a turn end even on failure, got {event:?}"
        );
    }

    #[tokio::test]
    async fn connect_negotiates_matching_protocol_version() {
        let (client, _events) = AcpClient::connect(fake_agent(PROTOCOL_VERSION, true))
            .await
            .expect("connect");

        assert!(client.capabilities().supports_resume);
    }

    #[tokio::test]
    async fn connect_fails_closed_on_unsupported_protocol_version() {
        let result = AcpClient::connect(fake_agent(PROTOCOL_VERSION + 1, false)).await;

        assert!(matches!(
            result,
            Err(AcpError::UnsupportedProtocolVersion { agent, client }) if agent == PROTOCOL_VERSION + 1 && client == PROTOCOL_VERSION
        ));
    }

    #[tokio::test]
    async fn session_new_returns_session_id() {
        let (client, _events) = AcpClient::connect(fake_agent(PROTOCOL_VERSION, true))
            .await
            .expect("connect");

        let session = client
            .session_new("/tmp/project", None)
            .await
            .expect("session id");

        assert_eq!(session.session_id, "sess-1");
    }

    /// A fake agent that appends every line it receives to `log_path`, so
    /// the test can inspect the raw `session/new`/`session/load` params it
    /// was sent. `declares_http` controls whether its `initialize`
    /// response advertises `mcpCapabilities.http` - required before an
    /// `http`-type `mcpServers` entry is valid per the ACP spec.
    fn logging_fake_agent(log_path: &std::path::Path, declares_http: bool) -> Command {
        let mut command = Command::new("sh");
        let mcp_capabilities = if declares_http {
            r#"{\"http\":true}"#
        } else {
            r#"{}"#
        };
        let script = format!(
            r#"while IFS= read -r line; do
              echo "$line" >> '{}'
              id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
              case "$method" in
                initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{\"mcpCapabilities\":{mcp_capabilities}}}}}}}" ;;
                session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}" ;;
                *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
              esac
            done"#,
            log_path.display()
        );
        command.arg("-c").arg(script);
        command
    }

    #[tokio::test]
    async fn session_new_carries_the_knot_mcp_server_when_enabled_and_supported() {
        let log = tempfile::NamedTempFile::new().unwrap();
        let (client, _events) = AcpClient::connect(logging_fake_agent(log.path(), true))
            .await
            .expect("connect");
        client
            .session_new("/tmp/project", Some("http://127.0.0.1:8767/mcp"))
            .await
            .expect("session");

        let log = std::fs::read_to_string(log.path()).unwrap();
        let request_line = log
            .lines()
            .find(|line| line.contains("session/new"))
            .expect("session/new request logged");
        assert!(request_line.contains(r#""type":"http""#));
        assert!(request_line.contains(r#""name":"knot""#));
        assert!(request_line.contains(r#""url":"http://127.0.0.1:8767/mcp""#));
        assert!(request_line.contains(r#""headers":[]"#));
    }

    #[tokio::test]
    async fn session_new_omits_the_mcp_server_when_the_agent_does_not_support_http() {
        let log = tempfile::NamedTempFile::new().unwrap();
        let (client, _events) = AcpClient::connect(logging_fake_agent(log.path(), false))
            .await
            .expect("connect");
        client
            .session_new("/tmp/project", Some("http://127.0.0.1:8767/mcp"))
            .await
            .expect("session");

        let log = std::fs::read_to_string(log.path()).unwrap();
        let request_line = log
            .lines()
            .find(|line| line.contains("session/new"))
            .expect("session/new request logged");
        assert!(request_line.contains(r#""mcpServers":[]"#));
    }

    #[tokio::test]
    async fn session_new_sends_no_mcp_servers_when_disabled() {
        let log = tempfile::NamedTempFile::new().unwrap();
        let (client, _events) = AcpClient::connect(logging_fake_agent(log.path(), true))
            .await
            .expect("connect");
        client
            .session_new("/tmp/project", None)
            .await
            .expect("session");

        let log = std::fs::read_to_string(log.path()).unwrap();
        let request_line = log
            .lines()
            .find(|line| line.contains("session/new"))
            .expect("session/new request logged");
        assert!(request_line.contains(r#""mcpServers":[]"#));
    }

    #[tokio::test]
    async fn session_load_fails_closed_when_resume_unsupported() {
        let (client, _events) = AcpClient::connect(fake_agent(PROTOCOL_VERSION, false))
            .await
            .expect("connect");

        let result = client.session_load("sess-1", "/tmp/project", None).await;

        assert!(matches!(result, Err(AcpError::ResumeNotSupported)));
    }

    #[tokio::test]
    async fn subprocess_exit_ends_session_and_resolves_pending_request_with_error() {
        // A subprocess that answers `initialize` then exits immediately,
        // simulating a crash mid-turn: the next request must resolve with
        // an error rather than hang.
        let mut command = Command::new("sh");
        command.arg("-c").arg(
            r#"read -r line
              id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}""#,
        );
        let (client, mut events) = AcpClient::connect(command).await.expect("connect");

        let result = client.session_new("/tmp/project", None).await;

        assert!(result.is_err());
        let ended = events.recv().await.expect("ended event");
        assert!(matches!(
            ended,
            SessionEvent::Ended(SessionEndCause::ProcessExited { .. })
        ));
    }

    /// Fake agent that, once `session/new` succeeds, streams a text delta
    /// and a turn-end update, then sends a `session/request_permission`
    /// request; on receiving the client's response it echoes the chosen
    /// option back as one more text delta, so the test can assert the
    /// decision actually reached the agent.
    fn permission_flow_agent() -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(
            r#"while IFS= read -r line; do
              id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
              case "$method" in
                initialize)
                  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
                  ;;
                session/new)
                  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}"
                  echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"hello\"}}"
                  echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"turn_end\",\"stopReason\":\"end_turn\"}}"
                  echo "{\"jsonrpc\":\"2.0\",\"id\":100,\"method\":\"session/request_permission\",\"params\":{\"toolCall\":{\"toolCallId\":\"tc1\"},\"options\":[{\"optionId\":\"allow-once\",\"name\":\"Allow\"},{\"optionId\":\"deny\",\"name\":\"Deny\"}]}}"
                  ;;
                "")
                  pid=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                  if [ "$pid" = "100" ]; then
                    opt=$(echo "$line" | sed -E 's/.*"optionId":"([^"]+)".*/\1/')
                    echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"decision:$opt\"}}"
                  fi
                  ;;
              esac
            done"#,
        );
        command
    }

    #[tokio::test]
    async fn close_while_permission_pending_ends_session_without_hanging() {
        let (client, mut events) = AcpClient::connect(permission_flow_agent())
            .await
            .expect("connect");
        client
            .session_new("/tmp/project", None)
            .await
            .expect("session id");
        let _text = events.recv().await.expect("text delta");
        let _turn_end = events.recv().await.expect("turn end");
        let _permission = events.recv().await.expect("permission request");

        client.close().await;

        // Per the "session closed while a permission request is pending"
        // scenario, the still-pending decision auto-resolves to Deny
        // (rather than hanging) as part of tearing the session down.
        let ended = events.recv().await.expect("ended event");
        assert!(matches!(ended, SessionEvent::Ended(_)));
    }

    #[tokio::test]
    async fn ordered_updates_stream_text_and_turn_end() {
        let (client, mut events) = AcpClient::connect(permission_flow_agent())
            .await
            .expect("connect");
        client
            .session_new("/tmp/project", None)
            .await
            .expect("session id");

        let first = events.recv().await.expect("first update");
        let second = events.recv().await.expect("second update");

        assert!(
            matches!(first, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "hello")
        );
        assert!(
            matches!(second, SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }) if stop_reason == "end_turn")
        );
    }

    #[tokio::test]
    async fn permission_deny_decision_is_delivered_to_the_agent() {
        let (client, mut events) = AcpClient::connect(permission_flow_agent())
            .await
            .expect("connect");
        client
            .session_new("/tmp/project", None)
            .await
            .expect("session id");
        let _text = events.recv().await.expect("text delta");
        let _turn_end = events.recv().await.expect("turn end");
        let permission = match events.recv().await.expect("permission request") {
            SessionEvent::PermissionRequest(request) => request,
            other => panic!("expected a permission request, got {other:?}"),
        };

        client.answer_permission(&permission, PermissionDecision::Deny);

        let confirmation = events.recv().await.expect("decision echoed back");
        assert!(
            matches!(confirmation, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "decision:deny")
        );
    }

    /// Fake agent declaring one `select` Session Config Option ("mode") on
    /// `session/new`, and answering `session/set_config_option` with an
    /// updated `currentValue`.
    fn config_options_agent() -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(
            r#"while IFS= read -r line; do
              id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
              case "$method" in
                initialize)
                  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
                  ;;
                session/new)
                  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\",\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"category\":\"mode\",\"type\":\"select\",\"currentValue\":\"ask\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
                  ;;
                session/set_config_option)
                  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"category\":\"mode\",\"type\":\"select\",\"currentValue\":\"code\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
                  ;;
                *) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}" ;;
              esac
            done"#,
        );
        command
    }

    #[tokio::test]
    async fn session_new_parses_declared_config_options() {
        let (client, _events) = AcpClient::connect(config_options_agent())
            .await
            .expect("connect");

        let session = client
            .session_new("/tmp/project", None)
            .await
            .expect("session");

        assert_eq!(session.config_options.len(), 1);
        assert_eq!(session.config_options[0].id, "mode");
        assert_eq!(session.config_options[0].category.as_deref(), Some("mode"));
        assert_eq!(session.config_options[0].options.len(), 2);
    }

    #[tokio::test]
    async fn set_config_option_sends_the_selection_and_returns_the_updated_list() {
        let (client, _events) = AcpClient::connect(config_options_agent())
            .await
            .expect("connect");
        let session = client
            .session_new("/tmp/project", None)
            .await
            .expect("session");

        let updated = client
            .session_set_config_option(&session.session_id, "mode", "code")
            .await
            .expect("set config option");

        assert_eq!(updated[0].current_value, serde_json::json!("code"));
    }
}
