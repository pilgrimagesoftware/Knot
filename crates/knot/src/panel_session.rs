//! Manages a Panel-mode agent's live ACP connection: starts
//! `knot_terminal::AcpSession`, drains its event stream into
//! `panel_state::PanelState`, and tracks a dirty flag so the UI poll loop
//! knows when to repaint - mirrors `Grid::take_dirty`'s pattern for the
//! terminal view.

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
                PanelPhase::Connecting(progress.lock()
                                               .map(|step| *step)
                                               .unwrap_or(ConnectStep::Starting { program:
                                                                                      "the agent", }))
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
