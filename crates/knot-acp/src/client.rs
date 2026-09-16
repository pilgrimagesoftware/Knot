//! ACP semantics (`initialize`, session lifecycle, updates, permissions)
//! built on top of the generic [`Transport`].

use std::sync::Arc;
use std::sync::Mutex;

use serde_json::{Value, json};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::error::{AcpError, Result, SessionEndCause};
use crate::protocol::{
    AgentCapabilities, InitializeParams, InitializeResult, JsonRpcErrorPayload, PROTOCOL_VERSION,
    PermissionDecision, PermissionOption, PermissionRequest, SessionUpdate,
};
use crate::transport::{Transport, TransportEvent};

/// One event in a session's ordered stream: either an ACP `session/update`
/// or a `session/request_permission` the caller must answer, plus a
/// terminal `Ended` event when the session stops taking requests.
#[derive(Debug)]
pub enum SessionEvent {
    Update(SessionUpdate),
    PermissionRequest(PermissionRequest),
    Ended(SessionEndCause),
}

pub struct AcpClient {
    transport:          Arc<Transport>,
    capabilities:       AgentCapabilities,
    permission_pending:
        Arc<Mutex<std::collections::HashMap<String, oneshot::Sender<PermissionDecision>>>>,
}

impl AcpClient {
    /// Spawns `command`, negotiates the protocol version, and returns the
    /// client plus the ordered session-event stream. Fails closed (per
    /// `acp-client`'s capability-negotiation requirement) if the agent
    /// reports an unsupported protocol version.
    pub async fn connect(command: Command)
                         -> Result<(Self, mpsc::UnboundedReceiver<SessionEvent>)> {
        let (transport, mut transport_events) = Transport::spawn(command)?;

        let init_params = InitializeParams { protocol_version: PROTOCOL_VERSION, };
        let raw = transport.request("initialize",
                                    Some(serde_json::to_value(init_params).expect("serializable")))
                           .await?;
        let init_result: InitializeResult = serde_json::from_value(raw).map_err(|error| {
                                                                           AcpError::Rpc {
                code: -32600,
                message: format!("malformed initialize response: {error}"),
            }
                                                                       })?;
        if init_result.protocol_version != PROTOCOL_VERSION {
            return Err(AcpError::UnsupportedProtocolVersion { client: PROTOCOL_VERSION,
                                                              agent:  init_result.protocol_version, });
        }

        let permission_pending: Arc<
            Mutex<std::collections::HashMap<String, oneshot::Sender<PermissionDecision>>>,
        > = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let (events_tx, events_rx) = mpsc::unbounded_channel();

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
                        let tool_call_id = params.get("toolCallId")
                                                 .and_then(Value::as_str)
                                                 .unwrap_or_default()
                                                 .to_owned();
                        let options: Vec<PermissionOption> = serde_json::from_value(
                            params
                                .get("options")
                                .cloned()
                                .unwrap_or(Value::Array(vec![])),
                        )
                        .unwrap_or_default();
                        let rpc_key = id.to_string();
                        let (decision_tx, decision_rx) = oneshot::channel();
                        pending_for_loop.lock()
                                        .expect("permission mutex poisoned")
                                        .insert(rpc_key.clone(), decision_tx);
                        let _ =
                            events_tx.send(SessionEvent::PermissionRequest(PermissionRequest {
                                rpc_id: id.clone(),
                                tool_call_id,
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
                        let pending: Vec<_> = pending_for_loop.lock()
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

        Ok((Self { transport,
                   capabilities: init_result.capabilities,
                   permission_pending },
            events_rx))
    }

    pub fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    pub async fn session_new(&self, cwd: &str) -> Result<String> {
        // `mcpServers` is required by at least the Gemini CLI adapter (it
        // rejects the request with an invalid_type validation error
        // without it, confirmed against a live `gemini --acp` handshake);
        // an empty array is the correct "no MCP config yet" value.
        let raw = self.transport
                      .request("session/new", Some(json!({ "cwd": cwd, "mcpServers": [] })))
                      .await?;
        session_id_from(&raw)
    }

    /// Resumes a prior session. Returns a typed "not supported" error
    /// without sending the request when the agent's capabilities don't
    /// advertise `session/load` support.
    pub async fn session_load(&self, session_id: &str, cwd: &str) -> Result<String> {
        if !self.capabilities.supports_resume {
            return Err(AcpError::ResumeNotSupported);
        }
        let raw = self.transport
                      .request("session/load",
                               Some(json!({ "sessionId": session_id, "cwd": cwd,
                                          "mcpServers": [] })))
                      .await?;
        session_id_from(&raw)
    }

    pub async fn session_prompt(&self, session_id: &str, text: &str) -> Result<()> {
        self.transport
            .request("session/prompt", Some(json!({ "sessionId": session_id, "prompt": [{ "type": "text", "text": text }] })))
            .await?;
        Ok(())
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
        if let Some(sender) = self.permission_pending
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
       .ok_or_else(|| AcpError::Rpc { code:    -32600,
                                      message: "session response missing sessionId".to_owned(), })
}

fn permission_result(decision: PermissionDecision, options: &[PermissionOption])
                     -> std::result::Result<Value, JsonRpcErrorPayload> {
    let outcome = match decision {
        PermissionDecision::Allow => options.first()
                                            .map(|option| option.option_id.clone())
                                            .unwrap_or_else(|| "allow".to_owned()),
        PermissionDecision::Deny => options.iter()
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

    #[tokio::test]
    async fn connect_negotiates_matching_protocol_version() {
        let (client, _events) =
            AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
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
        let (client, _events) =
            AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                                  .expect("connect");

        let session_id = client.session_new("/tmp/project")
                               .await
                               .expect("session id");

        assert_eq!(session_id, "sess-1");
    }

    #[tokio::test]
    async fn session_load_fails_closed_when_resume_unsupported() {
        let (client, _events) =
            AcpClient::connect(fake_agent(PROTOCOL_VERSION, false)).await
                                                                   .expect("connect");

        let result = client.session_load("sess-1", "/tmp/project").await;

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

        let result = client.session_new("/tmp/project").await;

        assert!(result.is_err());
        let ended = events.recv().await.expect("ended event");
        assert!(matches!(ended,
                         SessionEvent::Ended(SessionEndCause::ProcessExited { .. })));
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
                  echo "{\"jsonrpc\":\"2.0\",\"id\":100,\"method\":\"session/request_permission\",\"params\":{\"toolCallId\":\"tc1\",\"options\":[{\"optionId\":\"allow-once\",\"name\":\"Allow\"},{\"optionId\":\"deny\",\"name\":\"Deny\"}]}}"
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
        let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                              .expect("connect");
        client.session_new("/tmp/project")
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
        let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                              .expect("connect");
        client.session_new("/tmp/project")
              .await
              .expect("session id");

        let first = events.recv().await.expect("first update");
        let second = events.recv().await.expect("second update");

        assert!(matches!(first, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "hello"));
        assert!(matches!(second, SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }) if stop_reason == "end_turn"));
    }

    #[tokio::test]
    async fn permission_deny_decision_is_delivered_to_the_agent() {
        let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                              .expect("connect");
        client.session_new("/tmp/project")
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
        assert!(matches!(confirmation, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "decision:deny"));
    }
}
