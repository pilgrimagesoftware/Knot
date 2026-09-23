//! ACP semantics (`initialize`, session lifecycle, updates, permissions)
//! built on top of the generic [`Transport`].

use std::sync::Arc;

use parking_lot::Mutex;
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
    transport:           Arc<Transport>,
    capabilities:        AgentCapabilities,
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
    events:              mpsc::UnboundedSender<SessionEvent>,
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
        let events_for_client = events_tx.clone();

        // `Weak`, not `Arc`: this pump only ends when the transport's
        // `events_tx` drops, and that cannot happen while the pump itself
        // holds the transport alive. Holding a strong reference here was
        // the second half of the cycle that orphaned adapter subprocesses
        // (see `Transport::tasks`).
        let transport_for_loop = Arc::downgrade(&transport);
        let pending_for_loop = Arc::clone(&permission_pending);
        tokio::spawn(async move {
            while let Some(event) = transport_events.recv().await {
                let Some(transport_for_loop) = transport_for_loop.upgrade()
                else {
                    break;
                };
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
                        let tool_call_id = params.get("toolCall")
                                                 .and_then(|call| call.get("toolCallId"))
                                                 .or_else(|| params.get("toolCallId"))
                                                 .and_then(Value::as_str)
                                                 .unwrap_or_default()
                                                 .to_owned();
                        let tool_call_title = params.get("toolCall")
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
                        pending_for_loop.lock().insert(rpc_key.clone(), decision_tx);
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
                        let pending: Vec<_> = pending_for_loop.lock().drain().collect();
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
                   init_config_options: init_result.config_options,
                   permission_pending,
                   events: events_for_client },
            events_rx))
    }

    pub fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    /// The adapter subprocess's process id, while the connection is live.
    ///
    /// A panel agent's session root: what the processes section enumerates
    /// descendants of. `None` once the adapter has been reaped.
    pub fn process_id(&self) -> Option<u32> {
        self.transport.process_id()
    }

    pub async fn session_new(&self, cwd: &str, mcp_url: Option<&str>) -> Result<NewSession> {
        let raw = self.transport
                      .request("session/new",
                               Some(json!({ "cwd": cwd,
                                          "mcpServers": self.mcp_servers(mcp_url) })))
                      .await?;
        let session_id = session_id_from(&raw)?;
        let config_options = config_options_from(&raw, &self.init_config_options);
        Ok(NewSession { session_id,
                        config_options })
    }

    /// Resumes a prior session. Returns a typed "not supported" error
    /// without sending the request when the agent's capabilities don't
    /// advertise `session/load` support.
    pub async fn session_load(&self, session_id: &str, cwd: &str, mcp_url: Option<&str>)
                              -> Result<NewSession> {
        if !self.capabilities.supports_resume {
            return Err(AcpError::ResumeNotSupported);
        }
        let raw = self.transport
                      .request("session/load",
                               Some(json!({ "sessionId": session_id, "cwd": cwd,
                                          "mcpServers": self.mcp_servers(mcp_url) })))
                      .await?;
        let session_id = session_id_from(&raw)?;
        let config_options = config_options_from(&raw, &self.init_config_options);
        Ok(NewSession { session_id,
                        config_options })
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
    pub async fn session_set_config_option(&self, session_id: &str, config_id: &str, value: &str)
                                           -> Result<Vec<ConfigOption>> {
        let raw = self.transport
                      .request("session/set_config_option",
                               Some(json!({ "sessionId": session_id, "configId": config_id,
                                          "type": "id", "value": value })))
                      .await?;
        Ok(raw.get("configOptions")
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
            Ok(raw) => raw.get("stopReason")
                          .and_then(Value::as_str)
                          .unwrap_or("end_turn")
                          .to_owned(),
            Err(_) => "error".to_owned(),
        };
        let _ = self.events
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
        if let Some(sender) = self.permission_pending.lock().remove(&key) {
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
mod tests;
