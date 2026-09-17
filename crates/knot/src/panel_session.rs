//! Manages a Panel-mode agent's live ACP connection: starts
//! `knot_terminal::AcpSession`, drains its event stream into
//! `panel_state::PanelState`, and tracks a dirty flag so the UI poll loop
//! knows when to repaint - mirrors `Grid::take_dirty`'s pattern for the
//! terminal view.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use knot_acp::{PermissionDecision, PermissionRequest, Result as AcpResult, SessionEvent};
use knot_agent_launch::AdapterLaunch;
use knot_terminal::AcpSession;

use crate::panel_state::PanelState;

pub struct PanelSessionHandle {
    session: AcpSession,
    state:   Arc<Mutex<PanelState>>,
    dirty:   Arc<AtomicBool>,
}

impl PanelSessionHandle {
    /// Starts the adapter connection and spawns a background task (on the
    /// current tokio runtime) draining its event stream into `state` until
    /// the session ends.
    pub async fn start(launch: &AdapterLaunch, cwd: &str, prior_session_id: Option<&str>)
                       -> AcpResult<Self> {
        let (session, mut events) = AcpSession::start(launch, cwd, prior_session_id).await?;
        let state = Arc::new(Mutex::new(PanelState::new()));
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

    pub async fn stop(&self) {
        self.session.stop().await;
    }
}

/// A Panel-mode agent's connection lifecycle: connecting, ready to use, or
/// failed to connect (per `acp-client`'s "fails closed to Terminal mode
/// with a visible error" requirement - the caller renders `Failed`'s
/// message rather than hanging on `Connecting` forever).
pub enum PanelSessionSlot {
    Connecting,
    Ready(PanelSessionHandle),
    Failed(String),
}
