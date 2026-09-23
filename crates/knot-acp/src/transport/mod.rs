//! JSON-RPC 2.0 framing over a subprocess's stdio.
//!
//! Deliberately has no ACP-specific knowledge (method names, session
//! semantics) - it pairs requests with responses by id and hands every
//! other incoming message (server-to-client requests, notifications) to the
//! caller as an ordered event stream. `AcpClient` builds ACP semantics on
//! top of this.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};

use parking_lot::Mutex;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex as AsyncMutex, mpsc, oneshot};

use crate::error::{AcpError, Result, SessionEndCause};
use crate::protocol::{
    IncomingMessage, JsonRpcErrorPayload, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};

mod events;
pub use events::TransportEvent;

pub struct Transport {
    child:     Mutex<Child>,
    stdin:     AsyncMutex<ChildStdin>,
    next_id:   AtomicI64,
    pending: Mutex<HashMap<i64, oneshot::Sender<std::result::Result<Value, JsonRpcErrorPayload>>>>,
    events_tx: mpsc::UnboundedSender<TransportEvent>,
    /// The spawned program's name, for prefixing request/response logs so
    /// concurrent adapter connections can be told apart in stderr.
    program:   String,
    /// The stdout and stderr pumps, aborted when the transport drops.
    ///
    /// Neither task may hold a strong reference back to the transport: the
    /// `Child` lives in this struct with `kill_on_drop`, so a task holding
    /// an `Arc<Self>` keeps the subprocess alive forever - the reader waits
    /// for a pipe that only closes once the child is killed, and the child
    /// is only killed once the reader lets go. That cycle is why closing a
    /// window left one orphaned adapter process per panel agent.
    tasks:     Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl Drop for Transport {
    fn drop(&mut self) {
        for task in self.tasks.lock().drain(..) {
            task.abort();
        }
    }
}

impl Transport {
    /// Spawns `command` and starts the background read loop over its
    /// stdout. Returns the transport plus the event receiver for
    /// server-to-client requests, notifications, and session-end.
    pub fn spawn(mut command: Command)
                 -> Result<(std::sync::Arc<Self>, mpsc::UnboundedReceiver<TransportEvent>)> {
        let program = command.as_std()
                             .get_program()
                             .to_string_lossy()
                             .into_owned();
        let args = command.as_std()
                          .get_args()
                          .map(|arg| arg.to_string_lossy().into_owned())
                          .collect::<Vec<_>>()
                          .join(" ");
        eprintln!("knot-acp: [{program}] spawning {program} {args}");
        command.stdin(Stdio::piped())
               .stdout(Stdio::piped())
               // Piped (not discarded) and forwarded to our own stderr,
               // prefixed by the adapter's command name - an adapter that
               // fails a request often explains why on stderr, not as a
               // JSON-RPC error, and silently discarding it (the prior
               // behavior) made every such failure look like a hang.
               .stderr(Stdio::piped())
               // Without this, dropping the `Child` (e.g. the owning
               // window closing without an explicit `close()`/`stop()`
               // call) leaves the adapter subprocess running as an orphan
               // instead of terminating it.
               .kill_on_drop(true);
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");
        let (events_tx, events_rx) = mpsc::unbounded_channel();

        let stderr_program = program.clone();
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("[{stderr_program}] {line}");
            }
        });

        let transport = std::sync::Arc::new(Self { child: Mutex::new(child),
                                                   stdin: AsyncMutex::new(stdin),
                                                   next_id: AtomicI64::new(1),
                                                   pending: Mutex::new(HashMap::new()),
                                                   events_tx,
                                                   program,
                                                   tasks: Mutex::new(Vec::new()) });

        let reader_transport = std::sync::Arc::downgrade(&transport);
        let reader_task = tokio::spawn(async move {
            Self::read_loop(reader_transport, stdout).await;
        });
        {
            let mut tasks = transport.tasks.lock();
            tasks.push(stderr_task);
            tasks.push(reader_task);
        }

        Ok((transport, events_rx))
    }

    /// Pumps the child's stdout until the pipe closes or the transport goes
    /// away.
    ///
    /// Takes a `Weak`, not an `Arc`: see `Transport::tasks`. The upgrade
    /// happens per line rather than once, so the last owner dropping ends
    /// the loop at the next line instead of pinning the subprocess.
    async fn read_loop(transport: std::sync::Weak<Self>, stdout: tokio::process::ChildStdout) {
        let mut lines = BufReader::new(stdout).lines();
        loop {
            let next = lines.next_line().await;
            let Some(transport) = transport.upgrade()
            else {
                return;
            };
            match next {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    transport.handle_line(&line);
                }
                Ok(None) => {
                    let cause = transport.exit_cause().await;
                    transport.end_session(cause);
                    break;
                }
                Err(_) => {
                    transport.end_session(SessionEndCause::BrokenPipe);
                    break;
                }
            }
        }
    }

    /// The session root for a panel agent: the adapter subprocess, while it
    /// runs.
    ///
    /// What the processes section enumerates descendants of. `None` once the
    /// child has been reaped -- `tokio` drops the id at that point, which is
    /// exactly the "a stopped agent has no session root" the spec asks for.
    pub fn process_id(&self) -> Option<u32> {
        self.child.lock().id()
    }

    async fn exit_cause(&self) -> SessionEndCause {
        let status = self.child.lock().try_wait();
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
            let sender = self.pending.lock().remove(&id);
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
        let pending: Vec<_> = self.pending.lock().drain().collect();
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
        let mut child = self.child.lock();
        let _ = child.start_kill();
    }

    pub async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.send_request_id();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        let request = JsonRpcRequest { jsonrpc: "2.0",
                                       id,
                                       method: method.to_owned(),
                                       params };
        let line =
            serde_json::to_string(&request).map_err(|error| AcpError::Rpc { code:    -32700,
                                             message: error.to_string(), })?;
        let program = &self.program;
        eprintln!("knot-acp: [{program}] -> {method} (id {id})");
        self.write_line(line).await?;
        match rx.await {
            Ok(Ok(value)) => {
                eprintln!("knot-acp: [{program}] <- {method} (id {id}) ok");
                Ok(value)
            }
            Ok(Err(error)) => {
                eprintln!("knot-acp: [{program}] <- {method} (id {id}) error: {} {}",
                          error.code, error.message);
                Err(AcpError::Rpc { code:    error.code,
                                    message: error.message, })
            }
            Err(_) => {
                eprintln!("knot-acp: [{program}] <- {method} (id {id}) connection closed before a response");
                Err(AcpError::ConnectionClosed)
            }
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

    #[tokio::test]
    async fn the_adapter_subprocess_reports_its_pid() {
        let (transport, _events) = Transport::spawn(echo_command()).expect("spawn");

        let pid = transport.process_id()
                           .expect("a live adapter has a process id");

        assert!(pid > 1, "got {pid}");
        // The spawned `sh` really is this process's child, which is what
        // makes it usable as a session root.
        assert_ne!(pid, std::process::id());
    }
}
