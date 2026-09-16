//! JSON-RPC 2.0 framing over a subprocess's stdio.
//!
//! Deliberately has no ACP-specific knowledge (method names, session
//! semantics) - it pairs requests with responses by id and hands every
//! other incoming message (server-to-client requests, notifications) to the
//! caller as an ordered event stream. `AcpClient` builds ACP semantics on
//! top of this.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex as AsyncMutex, mpsc, oneshot};

use crate::error::{AcpError, Result, SessionEndCause};
use crate::protocol::{
    IncomingMessage, JsonRpcErrorPayload, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};

/// One message the agent subprocess sent us that isn't a response to a
/// request we made.
#[derive(Debug, Clone)]
pub enum TransportEvent {
    Request {
        id:     Value,
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
    Ended(SessionEndCause),
}

pub struct Transport {
    child:     Mutex<Child>,
    stdin:     AsyncMutex<ChildStdin>,
    next_id:   AtomicI64,
    pending: Mutex<HashMap<i64, oneshot::Sender<std::result::Result<Value, JsonRpcErrorPayload>>>>,
    events_tx: mpsc::UnboundedSender<TransportEvent>,
}

impl Transport {
    /// Spawns `command` and starts the background read loop over its
    /// stdout. Returns the transport plus the event receiver for
    /// server-to-client requests, notifications, and session-end.
    pub fn spawn(mut command: Command)
                 -> Result<(std::sync::Arc<Self>, mpsc::UnboundedReceiver<TransportEvent>)> {
        command.stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::null());
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");
        let (events_tx, events_rx) = mpsc::unbounded_channel();

        let transport = std::sync::Arc::new(Self { child: Mutex::new(child),
                                                   stdin: AsyncMutex::new(stdin),
                                                   next_id: AtomicI64::new(1),
                                                   pending: Mutex::new(HashMap::new()),
                                                   events_tx });

        let reader_transport = std::sync::Arc::clone(&transport);
        tokio::spawn(async move {
            reader_transport.read_loop(stdout).await;
        });

        Ok((transport, events_rx))
    }

    async fn read_loop(self: std::sync::Arc<Self>, stdout: tokio::process::ChildStdout) {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    self.handle_line(&line);
                }
                Ok(None) => {
                    self.end_session(self.exit_cause().await);
                    break;
                }
                Err(_) => {
                    self.end_session(SessionEndCause::BrokenPipe);
                    break;
                }
            }
        }
    }

    async fn exit_cause(&self) -> SessionEndCause {
        let status = self.child.lock().expect("child mutex poisoned").try_wait();
        match status {
            Ok(Some(status)) => SessionEndCause::ProcessExited { code: status.code(), },
            _ => SessionEndCause::ProcessExited { code: None },
        }
    }

    fn handle_line(&self, line: &str) {
        // Per the transport requirement, a batched array or a single
        // message are both valid; a malformed line is discarded and logged
        // rather than tearing down the connection.
        let messages: Vec<IncomingMessage> = match serde_json::from_str::<Value>(line) {
            Ok(Value::Array(items)) => items.into_iter()
                                            .filter_map(|item| serde_json::from_value(item).ok())
                                            .collect(),
            Ok(value) => match serde_json::from_value(value) {
                Ok(message) => vec![message],
                Err(_) => {
                    eprintln!("knot-acp: discarding malformed message: {line}");
                    Vec::new()
                }
            },
            Err(_) => {
                eprintln!("knot-acp: discarding non-json line: {line}");
                Vec::new()
            }
        };
        for message in messages {
            self.dispatch(message);
        }
    }

    fn dispatch(&self, message: IncomingMessage) {
        if message.is_response() {
            let Some(id) = message.id.as_ref().and_then(Value::as_i64)
            else {
                return;
            };
            let sender = self.pending
                             .lock()
                             .expect("pending mutex poisoned")
                             .remove(&id);
            if let Some(sender) = sender {
                let outcome = match message.error {
                    Some(error) => Err(error),
                    None => Ok(message.result.unwrap_or(Value::Null)),
                };
                let _ = sender.send(outcome);
            }
        }
        else if message.is_request() {
            let _ =
                self.events_tx
                    .send(TransportEvent::Request { id:     message.id
                                                                   .expect("checked by is_request"),
                                                    method: message.method
                                                                   .expect("checked by is_request"),
                                                    params: message.params, });
        }
        else if message.is_notification() {
            let _ = self.events_tx.send(TransportEvent::Notification {
                method: message.method.expect("checked by is_notification"),
                params: message.params,
            });
        }
    }

    fn end_session(&self, cause: SessionEndCause) {
        let pending: Vec<_> = self.pending
                                  .lock()
                                  .expect("pending mutex poisoned")
                                  .drain()
                                  .collect();
        for (_, sender) in pending {
            let _ = sender.send(Err(JsonRpcErrorPayload { code:    -1,
                                                          message: cause.to_string(),
                                                          data:    None, }));
        }
        let _ = self.events_tx.send(TransportEvent::Ended(cause));
    }

    async fn write_line(&self, line: String) -> Result<()> {
        let bytes = format!("{line}\n").into_bytes();
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(&bytes).await.map_err(AcpError::Spawn)
    }

    pub fn send_request_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Terminates the subprocess. Any pending requests resolve with an
    /// error via the normal exit path once the read loop observes stdout
    /// close.
    pub async fn close(&self) {
        let mut child = self.child.lock().expect("child mutex poisoned");
        let _ = child.start_kill();
    }

    pub async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.send_request_id();
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .expect("pending mutex poisoned")
            .insert(id, tx);
        let request = JsonRpcRequest { jsonrpc: "2.0",
                                       id,
                                       method: method.to_owned(),
                                       params };
        let line =
            serde_json::to_string(&request).map_err(|error| AcpError::Rpc { code:    -32700,
                                             message: error.to_string(), })?;
        self.write_line(line).await?;
        match rx.await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(AcpError::Rpc { code:    error.code,
                                                  message: error.message, }),
            Err(_) => Err(AcpError::ConnectionClosed),
        }
    }

    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = JsonRpcNotification { jsonrpc: "2.0",
                                                 method: method.to_owned(),
                                                 params };
        let line = serde_json::to_string(&notification).map_err(|error| {
                                                           AcpError::Rpc { code:    -32700,
                                                                           message:
                                                                               error.to_string(), }
                                                       })?;
        self.write_line(line).await
    }

    pub async fn respond(&self, id: Value,
                         result: std::result::Result<Value, JsonRpcErrorPayload>)
                         -> Result<()> {
        let response = match result {
            Ok(result) => JsonRpcResponse { jsonrpc: "2.0",
                                            id,
                                            result: Some(result),
                                            error: None },
            Err(error) => JsonRpcResponse { jsonrpc: "2.0",
                                            id,
                                            result: None,
                                            error: Some(error) },
        };
        let line =
            serde_json::to_string(&response).map_err(|error| AcpError::Rpc { code:    -32700,
                                             message: error.to_string(), })?;
        self.write_line(line).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake ACP "agent": reads each JSON-RPC request line and echoes back
    /// a matching response, so tests can exercise request/response pairing
    /// without a real ACP-speaking subprocess.
    fn echo_command() -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(
            r#"while IFS= read -r line; do id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/'); echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"echoed\":true}}"; done"#,
        );
        command
    }

    #[tokio::test]
    async fn request_response_pairing() {
        let (transport, _events) = Transport::spawn(echo_command()).expect("spawn");

        let result = transport.request("ping", None).await.expect("response");

        assert_eq!(result, serde_json::json!({ "echoed": true }));
    }

    #[tokio::test]
    async fn concurrent_requests_pair_by_id() {
        let (transport, _events) = Transport::spawn(echo_command()).expect("spawn");

        let (first, second) =
            tokio::join!(transport.request("a", None), transport.request("b", None));

        assert_eq!(first.expect("first response"),
                   serde_json::json!({ "echoed": true }));
        assert_eq!(second.expect("second response"),
                   serde_json::json!({ "echoed": true }));
    }
}
