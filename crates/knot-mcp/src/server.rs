use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::consts;
use crate::hooks::{AgentHookHandler, HookRequest};
use crate::log::{Logger, Subject, Vitals, spawn_heartbeat};
use crate::rpc::{self, JsonRpcId, JsonRpcRequest, JsonRpcResponse};
use crate::session::McpSessionManager;
use crate::status::{self, AgentStatusEntry};
use crate::tools::ToolCatalog;

const SESSION_HEADER: &str = "Mcp-Session-Id";

/// Supplies the live agent snapshot the status endpoint serializes.
pub type AgentsSnapshotFn = Arc<dyn Fn() -> Vec<knot_agents::Agent> + Send + Sync>;

#[derive(Clone)]
struct AppState {
    catalog:  Arc<dyn ToolCatalog>,
    agents:   AgentsSnapshotFn,
    hooks:    Option<Arc<dyn AgentHookHandler>>,
    sessions: McpSessionManager,
    /// Requests served since the heartbeat last reported. Read with
    /// `swap(0)`, so a request arriving mid-tick is counted in exactly one
    /// interval rather than in both or neither.
    served:   Arc<AtomicU64>,
    /// `None` when the server was built without a log path - the crate's own
    /// tests, which assert on responses rather than on a file.
    log:      Option<Logger>,
}

/// The local MCP HTTP server: health/info, JSON-RPC `/mcp`, SSE `/mcp`, and
/// the `GET /api/v1/agent/status` status endpoint.
pub struct McpServer {
    port:       u16,
    state:      AppState,
    handle:     Option<JoinHandle<()>>,
    bound_addr: Option<std::net::SocketAddr>,
    /// When this server began serving, for the heartbeat's uptime. Set by
    /// `start`, cleared by `stop`, so uptime is the current run's rather
    /// than the process's.
    started_at: Option<Instant>,
    /// The log's writer task, released on stop so it can drain and exit.
    log_task:   Option<JoinHandle<()>>,
    /// The heartbeat, owned here and aborted on stop: unlike the writer it
    /// has no natural end, so nothing but an abort stops it.
    heartbeat:  Option<JoinHandle<()>>,
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl McpServer {
    pub fn new(port: u16, catalog: Arc<dyn ToolCatalog>, agents: AgentsSnapshotFn) -> Self {
        Self { port,
               state: AppState { catalog,
                                 agents,
                                 hooks: None,
                                 sessions: McpSessionManager::new(),
                                 served: Arc::new(AtomicU64::new(0)),
                                 log: None },
               handle: None,
               bound_addr: None,
               started_at: None,
               log_task: None,
               heartbeat: None }
    }

    pub fn with_hook_handler(mut self, handler: Arc<dyn AgentHookHandler>) -> Self {
        self.state.hooks = Some(handler);
        self
    }

    /// Writes this server's diagnostics to the log file at `path`, in
    /// addition to standard error, starting a writer for it.
    ///
    /// The path is the caller's to decide. `knot-mcp` has no business
    /// knowing where an application keeps its logs, and taking it as an
    /// argument is also what lets every test here write to a temporary
    /// directory instead of the real one.
    ///
    /// For a supervised server use [`Self::with_logger`] instead: a
    /// supervisor builds a fresh server per attempt, and this would start a
    /// writer per attempt, several of them appending to one file with
    /// separate byte counters. One writer per log is what makes rotation
    /// safe without locking.
    pub fn with_log(mut self, path: std::path::PathBuf) -> Self {
        let (logger, task) = Logger::spawn(path);
        self.state.log = Some(logger);
        self.log_task = Some(task);
        self
    }

    /// Writes this server's diagnostics through an existing log, owned by
    /// the caller.
    ///
    /// The handle is a sender; the writer behind it outlives any one server,
    /// which is what lets a restarted server go on appending to the same
    /// file rather than opening a second view of it.
    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.state.log = Some(logger);
        self
    }

    /// The log this server writes to, if it has one.
    pub fn log(&self) -> Option<&Logger> {
        self.state.log.as_ref()
    }

    pub fn with_default_port(catalog: Arc<dyn ToolCatalog>, agents: AgentsSnapshotFn) -> Self {
        Self::new(consts::DEFAULT_PORT, catalog, agents)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// The port actually bound after `start()`, once known (differs from
    /// `port()` when constructed with port `0`).
    pub fn bound_addr(&self) -> Option<std::net::SocketAddr> {
        self.bound_addr
    }

    pub async fn start(&mut self) -> crate::Result<()> {
        let router = build_router(self.state.clone());
        let addr: std::net::SocketAddr = ([127, 0, 0, 1], self.port).into();
        if let Some(log) = self.log() {
            log.info(Subject::Lifecycle, format!("binding {addr}"));
        }
        let listener = match TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(error) => {
                if let Some(log) = self.log() {
                    log.error(Subject::Lifecycle, format!("bind {addr} failed: {error}"));
                }
                return Err(crate::McpError::Bind(addr, error));
            }
        };
        self.bound_addr = Some(listener.local_addr().map_err(crate::McpError::Serve)?);
        self.started_at = Some(Instant::now());
        if let Some(log) = self.log()
           && let Some(bound) = self.bound_addr
        {
            log.info(Subject::Lifecycle, format!("bound {bound}"));
            self.heartbeat = Some(spawn_heartbeat(Vitals { log:      log.clone(),
                                                           addr:     bound,
                                                           started:  Instant::now(),
                                                           sessions: self.state.sessions.clone(),
                                                           served:   Arc::clone(&self.state.served),
                                                           interval: consts::HEARTBEAT_INTERVAL, }));
        }
        self.handle = Some(tokio::spawn(async move {
                               let _ = axum::serve(listener, router).await;
                           }));
        Ok(())
    }

    /// How long this server has been serving, or `None` when it is not.
    pub fn uptime(&self) -> Option<std::time::Duration> {
        self.started_at.map(|at| at.elapsed())
    }

    /// Requests served since this was last called, resetting the count.
    ///
    /// `swap` rather than a read and a store: a request landing between the
    /// two would otherwise be counted in both intervals or in neither.
    pub fn take_served(&self) -> u64 {
        self.state.served.swap(0, Ordering::Relaxed)
    }

    /// How many MCP sessions are live.
    pub fn live_sessions(&self) -> usize {
        self.state.sessions.len()
    }

    /// Lends the serve task's handle so a supervisor can await its end.
    ///
    /// A `JoinHandle` is itself a future and is `Unpin`, so the borrow can
    /// sit in a `tokio::select!` arm directly. Its completion is the only
    /// signal that covers every way the task can stop - returning,
    /// panicking, or being aborted - which is why supervision watches this
    /// rather than a channel the task would have to remember to send on.
    ///
    /// Additive: `start`, `stop` and `Drop` are unchanged, and a caller
    /// that never asks for the handle behaves exactly as before. `None`
    /// before `start` and after `stop`.
    pub fn serve_handle(&mut self) -> Option<&mut JoinHandle<()>> {
        self.handle.as_mut()
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        // Before the stop entry, so no heartbeat can land after it and
        // claim the server is still up.
        if let Some(heartbeat) = self.heartbeat.take() {
            heartbeat.abort();
        }
        if let Some(log) = self.log() {
            log.info(Subject::Lifecycle, "stopped");
        }
        self.bound_addr = None;
        self.started_at = None;
        // Dropping the sender, never aborting the writer.
        //
        // `Writer::run` ends when every sender is gone, so releasing this
        // one - and the clones the aborted serve task was holding - lets it
        // drain what is queued and exit on its own. Aborting it here would
        // cancel it at its next await with the "stopped" entry still in the
        // channel, losing the one line that explains why the log ends.
        //
        // That leaves the task briefly unowned, against this workspace's
        // usual rule. It is bounded: the task's only exit condition is the
        // channel closing, and this is what closes it.
        self.state.log = None;
        self.log_task = None;
    }
}

fn build_router(state: AppState) -> Router {
    Router::new().route("/health", get(health))
                 .route("/", get(info))
                 .route("/mcp", post(mcp_rpc).get(mcp_sse))
                 .route("/api/v1/agent/status",
                        get(agent_status_endpoint).post(agent_status_hook_endpoint))
                 .route("/api/v1/agent/register", post(agent_register_endpoint))
                 .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

async fn info() -> Response {
    axum::Json(serde_json::json!({
                   "name": consts::SERVER_NAME,
                   "version": consts::SERVER_VERSION,
               })).into_response()
}

async fn agent_status_endpoint(State(state): State<AppState>) -> axum::Json<Vec<AgentStatusEntry>> {
    let agents = (state.agents)();
    axum::Json(status::agent_status(&agents))
}

async fn agent_register_endpoint(State(state): State<AppState>, body: Bytes) -> Response {
    hook_response(state.hooks.as_deref(), &body, HookAction::Register)
}

async fn agent_status_hook_endpoint(State(state): State<AppState>, body: Bytes) -> Response {
    hook_response(state.hooks.as_deref(), &body, HookAction::Status)
}

enum HookAction {
    Register,
    Status,
}

fn hook_response(handler: Option<&dyn AgentHookHandler>, body: &[u8], action: HookAction)
                 -> Response {
    let request: HookRequest = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(error) => return (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    };
    if let Err(error) = request.agent_id().and(request.validate_agent()) {
        return (StatusCode::BAD_REQUEST, error.to_string()).into_response();
    }

    let Some(handler) = handler
    else {
        return (StatusCode::SERVICE_UNAVAILABLE, "Agent hooks are not configured").into_response();
    };
    let result = match action {
        HookAction::Register => handler.register(&request),
        HookAction::Status => handler.status(&request),
    };
    match result {
        Ok(value) => axum::Json(value).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

fn sse_response(event: &str, data: &str, session_id: Option<&str>) -> Response {
    let body = format!("event: {event}\ndata: {data}\n\n");
    let mut response = Response::builder().status(StatusCode::OK)
                                          .header(header::CONTENT_TYPE, "text/event-stream")
                                          .header(header::CACHE_CONTROL, "no-cache")
                                          .header(header::CONNECTION, "keep-alive");
    if let Some(id) = session_id
       && let Ok(value) = HeaderValue::from_str(id)
    {
        response = response.header(SESSION_HEADER, value);
    }
    response.body(axum::body::Body::from(body)).unwrap()
}

async fn mcp_sse() -> Response {
    sse_response("connected", "{\"status\":\"connected\"}", None)
}

async fn mcp_rpc(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    // Counted before parsing: a request that arrives malformed still
    // arrived, and a heartbeat reporting zero while a client hammers the
    // server with nonsense would be the more misleading of the two.
    state.served.fetch_add(1, Ordering::Relaxed);
    state.sessions.cleanup_stale_default();
    let session_id = headers.get(SESSION_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_string);
    let accepts_sse = headers.get(header::ACCEPT)
                             .and_then(|v| v.to_str().ok())
                             .is_some_and(|v| v.contains("text/event-stream"));

    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => return json_rpc_error_response(-32700, format!("Parse error: {e}")),
    };

    let response_session_id = if request.method == "initialize" {
        state.sessions.create_session(Uuid::nil()).id
    }
    else if let Some(session_id) = session_id {
        if state.sessions.session(&session_id).is_some() {
            state.sessions.touch(&session_id);
            session_id
        }
        else {
            // The client's session outlived the server's TTL (e.g. a paused
            // session resuming after `DEFAULT_SESSION_TIMEOUT`). Reissue a
            // fresh session instead of erroring so the client self-heals
            // without a manual `/mcp reconnect` or restart.
            state.sessions.create_session(Uuid::nil()).id
        }
    }
    else {
        state.sessions.create_session(Uuid::nil()).id
    };

    if request.method.starts_with("notifications/") {
        return StatusCode::ACCEPTED.into_response();
    }

    eprintln!("knot-mcp: [{response_session_id}] -> {}", request.method);
    if let Some(log) = state.log.as_ref() {
        log.info(Subject::Request,
                 format!("[{response_session_id}] {}", request.method));
    }
    let response = rpc::dispatch(&request, state.catalog.as_ref(), state.log.as_ref()).await;
    let outcome = if response.error.is_some() {
        "error"
    }
    else {
        "ok"
    };
    eprintln!("knot-mcp: [{response_session_id}] <- {} {outcome}",
              request.method);
    if let Some(log) = state.log.as_ref() {
        log.info(Subject::Response,
                 format!("[{response_session_id}] {} {outcome}", request.method));
    }

    if accepts_sse {
        let data = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
        return sse_response("message", &data, Some(&response_session_id));
    }

    let mut resp = axum::Json(response).into_response();
    if let Ok(value) = HeaderValue::from_str(&response_session_id) {
        resp.headers_mut().insert(SESSION_HEADER, value);
    }
    resp
}

fn json_rpc_error_response(code: i64, message: String) -> Response {
    let response = JsonRpcResponse::error(None::<JsonRpcId>, code, message);
    (StatusCode::BAD_REQUEST, axum::Json(response)).into_response()
}

#[cfg(test)]
mod tests;
