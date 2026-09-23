//! The MCP server's lifecycle state, as the rest of the application sees
//! it.
//!
//! Contract: `openspec/changes/supervise-mcp-server/specs/mcp-server/spec.md`
//! for the state itself, and the `desktop-notifications` delta for when a
//! failure is announced.
//!
//! The supervisor publishes its state over a `tokio::sync::watch` channel
//! on the MCP thread. GPUI cannot await that, and a desktop notification
//! must be raised on the main thread, so this sits between the two: a task
//! on the MCP runtime watches the channel and writes what it sees into two
//! shared places the main thread reads on its own schedule.
//!
//! - The **mirror** is a level: the settings pane reads it when it renders.
//! - The **failure slot** is an edge: one marker per failure episode, taken
//!   atomically so that with several windows open exactly one raises the
//!   notification. With no window open the marker waits in the slot, and the
//!   state row shows the failure the moment settings is opened.

use std::sync::Arc;

use knot_mcp::ServerState;
use parking_lot::Mutex;
use tokio::sync::watch;

/// Where the MCP server is, as last seen by the main thread.
pub(crate) type McpStateMirror = Arc<Mutex<ServerState>>;

/// A single slot holding at most one unclaimed failure marker.
pub(crate) type McpFailureSlot = Arc<Mutex<bool>>;

/// The MCP server's state and its pending failure marker, reachable from
/// any window.
///
/// A global for the same reason [`crate::app_support::AwaitingInput`] is
/// one: the settings pane and every workspace window need it, and the
/// supervisor runs off the main thread with no window to call.
#[derive(Clone)]
pub(crate) struct McpServerStatus {
    state:   McpStateMirror,
    failure: McpFailureSlot,
}

impl Default for McpServerStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServerStatus {
    /// A status nothing has reported into yet, which is the state of a
    /// server configuration has turned off.
    pub(crate) fn new() -> Self {
        Self { state:   Arc::new(Mutex::new(ServerState::Disabled)),
               failure: Arc::new(Mutex::new(false)), }
    }

    /// The state as last mirrored.
    pub(crate) fn state(&self) -> ServerState {
        self.state.lock().clone()
    }

    /// Takes the pending failure marker, answering whether there was one.
    ///
    /// The take is what makes this safe to call from every window: the
    /// first caller of a given episode gets `true` and every later one gets
    /// `false`, so the notification is raised exactly once however many
    /// windows are polling.
    pub(crate) fn claim_failure(&self) -> bool {
        std::mem::take(&mut *self.failure.lock())
    }

    /// Puts a failure marker in the slot, as [`mirror_server_state`] does
    /// when it sees an episode open. Tests only: they assert who claims a
    /// marker, separately from what puts one there.
    #[cfg(test)]
    pub(crate) fn mark_failed_for_test(&self) {
        *self.failure.lock() = true;
    }
}

impl gpui_kit::Global for McpServerStatus {}

/// The tag the MCP server's failure notification carries.
///
/// Deliberately not a UUID: `app_state::notification_response_agent_id`
/// reads a tag back as an agent id, and this notification is not tied to an
/// agent, so a click on it must find none to select.
pub(crate) const MCP_FAILURE_NOTIFICATION_TAG: &str = "mcp-server-failure";

/// Whether a window that just claimed a failure marker should raise the
/// notification.
///
/// The claim comes first and the setting second, deliberately: a failure
/// that happens while notifications are off is still consumed, so turning
/// them back on later does not deliver a stale announcement about a server
/// that has since recovered. The settings pane's state row reports the
/// failure either way.
pub(crate) fn should_notify_mcp_failure(desktop_notifications_enabled: bool, claimed: bool)
                                        -> bool {
    claimed && desktop_notifications_enabled
}

/// Tracks which failure episode has already been announced.
///
/// An episode is not "the state is retrying": supervision alternates
/// between retrying and starting while it keeps failing, so a rule that
/// looked only at the current state would announce every attempt. The
/// episode ends when the server reaches running - or when the application
/// stops it, which is not a failure at all.
#[derive(Debug, Default)]
pub(crate) struct FailureEpisodes {
    announced: bool,
}

impl FailureEpisodes {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Records `state`, answering whether it opens an episode that has not
    /// been announced yet.
    pub(crate) fn observe(&mut self, state: &ServerState) -> bool {
        match state {
            ServerState::Running { .. } | ServerState::Stopped | ServerState::Disabled => {
                self.announced = false;
                false
            }
            // Mid-episode: a retry in progress says nothing new.
            ServerState::Starting => false,
            ServerState::Retrying { .. } => {
                let first = !self.announced;
                self.announced = true;
                first
            }
        }
    }
}

/// Mirrors the supervisor's state for the main thread, for as long as the
/// supervisor publishes one.
///
/// Runs on the MCP thread's runtime beside [`knot_mcp::Supervisor::run`],
/// and returns when the supervisor drops its sender.
pub(crate) async fn mirror_server_state(mut states: watch::Receiver<ServerState>,
                                        status: McpServerStatus) {
    let mut episodes = FailureEpisodes::new();
    loop {
        {
            let state = states.borrow_and_update().clone();
            if episodes.observe(&state) {
                *status.failure.lock() = true;
            }
            *status.state.lock() = state;
        }
        if states.changed().await.is_err() {
            return;
        }
    }
}
