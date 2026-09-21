//! Manages a Panel-mode agent's live ACP connection: starts
//! `knot_terminal::AcpSession`, drains its event stream into
//! `panel_state::PanelState`, and tracks a dirty flag so the UI poll loop
//! knows when to repaint - mirrors `Grid::take_dirty`'s pattern for the
//! terminal view.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use knot_acp::{PermissionDecision, PermissionRequest, Result as AcpResult, SessionEvent};
use knot_agent_launch::AdapterConfig;
use knot_terminal::{AcpSession, ConnectProgress, ConnectStep};

use crate::panel_state::PanelState;

pub struct PanelSessionHandle {
    session: AcpSession,
    state:   Arc<Mutex<PanelState>>,
    dirty:   Arc<AtomicBool>,
}

impl PanelSessionHandle {
    /// Starts the adapter connection and spawns a background task (on the
    /// current tokio runtime) draining its event stream into `state` until
    /// the session ends. `mcp_url` is Knot's own MCP HTTP server URL, wired
    /// into the session when MCP is enabled.
    pub async fn start(config: &AdapterConfig, cwd: &str, prior_session_id: Option<&str>,
                       mcp_url: Option<&str>, progress: &ConnectProgress)
                       -> AcpResult<Self> {
        let (session, config_options, mut events) =
            AcpSession::start(config, cwd, prior_session_id, mcp_url, progress).await?;
        let mut initial_state = PanelState::new();
        initial_state.config_options = config_options;
        let state = Arc::new(Mutex::new(initial_state));
        let dirty = Arc::new(AtomicBool::new(false));

        let drain_state = Arc::clone(&state);
        let drain_dirty = Arc::clone(&dirty);
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                let ended = matches!(event, SessionEvent::Ended(_));
                if let Ok(mut state) = drain_state.lock() {
                    state.apply(event);
                }
                drain_dirty.store(true, Ordering::SeqCst);
                if ended {
                    break;
                }
            }
        });

        Ok(Self { session,
                  state,
                  dirty })
    }

    pub fn state(&self) -> Arc<Mutex<PanelState>> {
        Arc::clone(&self.state)
    }

    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }

    /// Whether new events arrived since the last call; clears the flag.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }

    // The window sends through `deliver_panel_prompt`, which also handles
    // queueing; this is the unqueued path, kept as the slot's own API.
    #[allow(dead_code)]
    pub async fn prompt(&self, text: &str) -> AcpResult<()> {
        self.session.prompt(text).await
    }

    /// Records a prompt the user just sent in the conversation state - the
    /// ACP stream itself never echoes it back, so the caller must add it
    /// explicitly before (or independent of) actually sending it.
    pub fn record_user_message(&self, text: String) {
        if let Ok(mut state) = self.state.lock() {
            state.push_user_message(text);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// A cheap clone of the underlying session, for a caller holding this
    /// handle behind a `Mutex` to `.await` on (e.g. `prompt`) without
    /// keeping the lock held across the await point.
    pub fn session(&self) -> AcpSession {
        self.session.clone()
    }

    /// A `'static` writer into this session's conversation, for the task
    /// that awaits a `prompt` after the handle - and the lock it came
    /// from - has been dropped. Without it a prompt that the agent
    /// refuses (an exhausted quota, a transport failure) has nowhere to
    /// report but stderr.
    pub fn recorder(&self) -> PanelRecorder {
        PanelRecorder { state: Arc::clone(&self.state),
                        dirty: Arc::clone(&self.dirty), }
    }

    pub fn answer_permission(&self, request: &PermissionRequest, decision: PermissionDecision) {
        self.session.answer_permission(request, decision);
        if let Ok(mut state) = self.state.lock() {
            state.resolve_permission();
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Turns off auto-scroll for the in-flight response, per the track
    /// toggle's "user scrolls away" scenario.
    pub fn clear_tracking(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.clear_tracking();
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Toggles auto-scroll for the in-flight response.
    pub fn toggle_tracking(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.toggle_tracking();
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Records the user opening or closing one tool-call card, per
    /// `acp-panel-ui`'s "The user's choice outlives the automatic one"
    /// requirement.
    pub fn toggle_tool_call(&self, id: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.toggle_tool_call(id);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Opens or closes one compact summary's run of tool calls.
    pub fn toggle_tool_run(&self, head_id: String) {
        if let Ok(mut state) = self.state.lock() {
            state.toggle_tool_run(head_id);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Sets auto-scroll directly, for the scroll-to-latest control.
    pub fn set_tracking(&self, tracking: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.set_tracking(tracking);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }

    /// Applies one Session Config Option selection (permission mode,
    /// model, reasoning effort, ...) and stores the agent's updated list.
    /// Returns an owned `'static` future rather than being `async fn`
    /// itself, so a caller with only `&mut App` (a popup-menu click
    /// handler, not a `Context<Self>` listener) can spawn it without
    /// holding this handle - or the session lock it came from - across
    /// the await point.
    pub fn set_config_option(&self, config_id: String, value: String)
                             -> impl std::future::Future<Output = ()> + Send + 'static {
        let session = self.session.clone();
        let state = Arc::clone(&self.state);
        let dirty = Arc::clone(&self.dirty);
        async move {
            if let Ok(config_options) = session.set_config_option(&config_id, &value).await {
                if let Ok(mut state) = state.lock() {
                    state.config_options = config_options;
                }
                dirty.store(true, Ordering::SeqCst);
            }
        }
    }

    pub async fn stop(&self) {
        self.session.stop().await;
    }

    // As `prompt` above: the window cancels via `stop_panel_prompt`.
    #[allow(dead_code)]
    pub async fn cancel(&self) -> AcpResult<()> {
        self.session.cancel().await
    }
}

/// Writes into a panel conversation without holding its handle: an owned
/// pair of the state and the repaint flag, so a spawned task can report a
/// failure back into the panel the user is looking at.
#[derive(Clone)]
pub struct PanelRecorder {
    state: Arc<Mutex<PanelState>>,
    dirty: Arc<AtomicBool>,
}

impl PanelRecorder {
    /// Shows `text` in the conversation as a failed turn.
    pub fn error(&self, text: String) {
        if let Ok(mut state) = self.state.lock() {
            state.push_error(text);
        }
        self.dirty.store(true, Ordering::SeqCst);
    }
}

/// A Panel-mode agent's connection lifecycle: connecting, ready to use, or
/// failed to connect (per `acp-client`'s "fails closed to Terminal mode
/// with a visible error" requirement - the caller renders `Failed`'s
/// message rather than hanging on `Connecting` forever).
pub enum PanelSessionSlot {
    /// Connecting, carrying the step the attempt is currently on so the
    /// placeholder can say what it's waiting for.
    Connecting(ConnectProgress),
    Ready(PanelSessionHandle),
    Failed(String),
}

impl PanelSessionSlot {
    /// A fresh `Connecting` slot and the progress cell the connect task
    /// writes to - the two halves of the same handle, so the caller can't
    /// accidentally hand the task a cell the slot isn't watching.
    pub fn connecting() -> (Self, ConnectProgress) {
        let progress: ConnectProgress =
            Arc::new(Mutex::new(ConnectStep::Starting { program: "the agent", }));
        (Self::Connecting(Arc::clone(&progress)), progress)
    }

    /// Which lifecycle phase this slot is in, without borrowing the
    /// handle. The UI's repaint poll compares this against the phase it
    /// last drew: the connecting task writes the slot from a background
    /// thread, and neither `Connecting` nor `Failed` carries a dirty flag
    /// of its own, so without this a failed connection would leave the
    /// pane showing "Connecting to agent…" forever.
    ///
    /// `Connecting` carries its step, so advancing through the connect
    /// sequence counts as a change and repaints too - that is what keeps
    /// the progress line live without a second dirty channel.
    pub fn phase(&self) -> PanelPhase {
        match self {
            Self::Connecting(progress) => {
                PanelPhase::Connecting(progress.lock().map(|step| *step).unwrap_or(
                    ConnectStep::Starting {
                        program: "the agent",
                    },
                ))
            }
            Self::Ready(_) => PanelPhase::Ready,
            Self::Failed(_) => PanelPhase::Failed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPhase {
    Connecting(ConnectStep),
    Ready,
    Failed,
}

/// What one panel connect attempt needs beyond the slot it publishes
/// into: the adapter to spawn, where to run it, and the first prompt to
/// send once it is live.
pub struct ConnectRequest<'a> {
    pub config:              &'a AdapterConfig,
    pub cwd:                 &'a str,
    pub prior_session_id:    Option<&'a str>,
    pub mcp_url:             Option<&'a str>,
    /// The registration prompt for a fresh session, or `None` when
    /// resuming (a resumed agent is already registered).
    pub registration_prompt: Option<String>,
    /// The agent's persisted session setup - config-option id -> value -
    /// replayed onto the new session before its first turn. Empty for an
    /// agent that has never had one chosen, which leaves the adapter's own
    /// defaults in place. See
    /// `openspec/specs/session-setup-persistence/spec.md`.
    pub session_config:      BTreeMap<String, String>,
}

/// Connects `request`'s adapter and drives `slot` through the connection
/// lifecycle, calling `on_session_id` with the opened session id so the
/// caller can persist it for a later resume.
pub async fn connect_into(slot: &Arc<Mutex<PanelSessionSlot>>, request: ConnectRequest<'_>,
                          progress: &ConnectProgress, on_session_id: impl FnOnce(&str)) {
    let handle = match PanelSessionHandle::start(request.config,
                                                 request.cwd,
                                                 request.prior_session_id,
                                                 request.mcp_url,
                                                 progress).await
    {
        Ok(handle) => handle,
        Err(error) => {
            *slot.lock().unwrap() = PanelSessionSlot::Failed(error.to_string());
            return;
        }
    };
    on_session_id(handle.session_id());

    // Replay the persisted setup before the first turn, so the turn runs
    // with the model, permission mode and effort the user last chose. An
    // option the adapter no longer declares is rejected by it and skipped
    // here rather than failing the connection - adapters add and drop
    // config options between versions.
    let mut restored_options = None;
    for (config_id, value) in &request.session_config {
        if let Ok(options) = handle.session().set_config_option(config_id, value).await {
            restored_options = Some(options);
        }
    }
    if let Some(options) = restored_options
       && let Ok(mut state) = handle.state().lock()
    {
        state.config_options = options;
    }

    // Publish the handle *before* sending the registration prompt.
    // `session/prompt` resolves only when the whole turn ends, and an
    // agent that asks permission during that turn (Gemini does, for its
    // first Knot MCP tool call) can only be answered through a `Ready`
    // slot - so awaiting the turn here first is a circular wait: the
    // turn needs a permission answer, the answer needs the slot, the
    // slot needs the turn. The registration prompt is recorded first so
    // the panel opens on the turn already in flight rather than blank.
    let session = handle.session();
    let recorder = handle.recorder();
    if let Some(prompt) = &request.registration_prompt {
        handle.record_user_message(prompt.clone());
    }
    *slot.lock().unwrap() = PanelSessionSlot::Ready(handle);

    if let Some(prompt) = request.registration_prompt
       && let Err(error) = session.prompt(&prompt).await
    {
        // Into the conversation, not just the console: the registration
        // turn is the first thing the panel shows, and an agent that
        // refuses it (an exhausted daily quota, say) otherwise leaves the
        // panel sitting on a prompt that never answers.
        recorder.error(format!("The agent could not start its first turn: {error}"));
        eprintln!("failed to send panel registration prompt: {error}");
    }
}

#[cfg(test)]
mod tests {
    use knot_agent_launch::AdapterConfig;

    use super::*;

    /// A fake adapter that completes the handshake but never answers
    /// `session/prompt` - standing in for an agent whose first turn is
    /// blocked (Gemini stalls its registration turn on a
    /// `session/request_permission` for the first Knot MCP tool call,
    /// which only a `Ready` slot can show and answer).
    fn stalling_prompt_adapter() -> AdapterConfig {
        AdapterConfig { command:                   "sh",
                        args:                      &[
                                                     "-c",
                                                     r#"while IFS= read -r line; do
                          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                          case "$method" in
                            initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
                            session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}" ;;
                          esac
                        done"#,
        ],
                        supports_resume:           false,
                        supports_permission_modes: false,
                        install:                   None, }
    }

    /// The registration turn must not gate the slot: an agent that asks
    /// permission mid-registration can only be answered through a `Ready`
    /// slot, so publishing the handle after the turn finishes deadlocks
    /// the connection.
    #[tokio::test]
    async fn the_slot_goes_ready_before_the_registration_turn_finishes() {
        let (connecting, progress) = PanelSessionSlot::connecting();
        let slot = Arc::new(Mutex::new(connecting));
        let request = ConnectRequest { config:              &stalling_prompt_adapter(),
                                       cwd:                 "/tmp/project",
                                       prior_session_id:    None,
                                       mcp_url:             None,
                                       registration_prompt: Some("register".to_string()),
                                       session_config:      BTreeMap::new(), };

        let watched = Arc::clone(&slot);
        let became_ready = async move {
            for _ in 0..200 {
                let phase = watched.lock().unwrap().phase();
                if phase == PanelPhase::Ready {
                    return true;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            false
        };

        tokio::select! {
            ready = became_ready => assert!(
                ready,
                "the slot must reach Ready while the registration turn is still in flight"
            ),
            () = connect_into(&slot, request, &progress, |_| {}) => {
                panic!("the stalled registration turn should never finish")
            }
        }
    }
}
