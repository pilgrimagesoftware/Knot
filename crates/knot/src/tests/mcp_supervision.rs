//! What the application does with the MCP server's supervised lifecycle:
//! the disabled case, and the failure marker the notification is raised
//! from.
//!
//! Contract: `openspec/changes/supervise-mcp-server/specs/mcp-server/spec.md`
//! and the change's `desktop-notifications` delta. The supervisor's own
//! behaviour is covered in `knot-mcp`; these cover the bridge into the
//! application.

use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::time::Duration;

use knot_mcp::ServerState;
use parking_lot::Mutex;

use crate::app_bootstrap::start_mcp_server;
use crate::app_state::notification_response_agent_id;
use crate::mcp_status::{
    FailureEpisodes, MCP_FAILURE_NOTIFICATION_TAG, McpServerStatus, mirror_server_state,
    should_notify_mcp_failure,
};

fn running() -> ServerState {
    ServerState::Running { addr: ([127, 0, 0, 1], 8767).into(), }
}

fn retrying(attempt: u32) -> ServerState {
    ServerState::Retrying { attempt,
                            next_delay: Duration::from_millis(500),
                            error: "address in use".to_string() }
}

/// A port nothing is listening on.
fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("an ephemeral port");
    let port = listener.local_addr().expect("bound").port();
    drop(listener);
    port
}

#[test]
fn a_disabled_server_binds_nothing_and_supervises_nothing() {
    let dir = tempfile::tempdir().expect("a temporary settings root");
    // `with_store_root` rather than `Settings::default`: a default-rooted
    // settings object in a test writes over the user's real configuration.
    let mut settings = knot_core::Settings::with_store_root(dir.path());
    settings.mcp_server_enabled = false;
    settings.mcp_server_port = free_port();
    let port = settings.mcp_server_port;

    let store = Arc::new(Mutex::new(crate::app_state::build_agent_store(&settings)));
    let (_stop, status) = start_mcp_server(store,
                                           knot_core::SharedSettings::new(settings),
                                           Arc::new(knot_messaging::QueuedNotifier::new()),
                                           Arc::new(Mutex::new(knot_messaging::MessageStore::new())),
                                           Arc::new(Mutex::new(Vec::new())),
                                           Arc::new(Mutex::new(Vec::new())),
                                           Arc::default());

    // Long enough for a supervisor that was going to start to have bound.
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(status.state(),
               ServerState::Disabled,
               "a disabled server is not supervised");
    assert!(!status.claim_failure(),
            "nothing failed, so nothing is announced");
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    TcpListener::bind(addr).expect("the configured port was never bound");
}

#[test]
fn one_episode_announces_once_however_many_retries_it_takes() {
    let mut episodes = FailureEpisodes::new();

    assert!(!episodes.observe(&ServerState::Starting),
            "a first start is not a failure");
    assert!(episodes.observe(&retrying(1)),
            "the failure opens an episode");
    // Supervision alternates between starting and retrying while it keeps
    // failing; none of that is a new episode.
    assert!(!episodes.observe(&ServerState::Starting));
    assert!(!episodes.observe(&retrying(2)));
    assert!(!episodes.observe(&ServerState::Starting));
    assert!(!episodes.observe(&retrying(3)));
}

#[test]
fn a_failure_after_a_recovery_announces_again() {
    let mut episodes = FailureEpisodes::new();

    assert!(episodes.observe(&retrying(1)));
    assert!(!episodes.observe(&retrying(2)));
    assert!(!episodes.observe(&running()),
            "reaching running ends the episode");
    assert!(episodes.observe(&retrying(1)),
            "a later failure is a new episode");
}

#[test]
fn an_intentional_stop_is_not_an_episode() {
    let mut episodes = FailureEpisodes::new();

    assert!(!episodes.observe(&ServerState::Stopped));
    assert!(!episodes.observe(&ServerState::Disabled));
    assert!(!episodes.observe(&running()));
}

#[test]
fn exactly_one_claimant_takes_a_failure_marker() {
    let status = McpServerStatus::new();
    assert!(!status.claim_failure(), "nothing has failed yet");

    // Two windows polling the same status, as two open windows do.
    let first_window = status.clone();
    let second_window = status.clone();
    status.mark_failed_for_test();

    assert!(first_window.claim_failure(),
            "the first window to poll raises the notification");
    assert!(!second_window.claim_failure(),
            "the second finds the slot already taken");
}

#[tokio::test]
async fn the_mirror_follows_the_supervisor_and_records_one_marker_per_episode() {
    let (states, receiver) = tokio::sync::watch::channel(ServerState::Starting);
    let status = McpServerStatus::new();
    let mirroring = tokio::spawn(mirror_server_state(receiver, status.clone()));

    for state in [retrying(1), ServerState::Starting, retrying(2), running()] {
        states.send_replace(state.clone());
        wait_until(|| status.state() == state,
                   "the mirror to follow the supervisor").await;
    }

    assert!(status.claim_failure(), "the episode was recorded");
    assert!(!status.claim_failure(), "and recorded only once");

    states.send_replace(retrying(1));
    wait_until(|| status.state().is_failing(), "the second failure").await;
    assert!(status.claim_failure(),
            "a failure after a recovery is announced again");

    mirroring.abort();
}

#[test]
fn the_failure_notification_is_gated_on_the_setting() {
    assert!(should_notify_mcp_failure(true, true),
            "a claimed marker with notifications on");
    assert!(!should_notify_mcp_failure(false, true),
            "notifications off raises nothing, though the state row still reports the failure");
    assert!(!should_notify_mcp_failure(true, false),
            "no marker, nothing to announce");
    assert!(!should_notify_mcp_failure(false, false));
}

#[test]
fn a_failure_while_notifications_are_off_is_still_consumed() {
    let status = McpServerStatus::new();
    status.mark_failed_for_test();

    // The window claims first and asks about the setting second, so a
    // marker cannot sit in the slot waiting to announce a server that has
    // since recovered.
    let claimed = status.claim_failure();
    assert!(!should_notify_mcp_failure(false, claimed));
    assert!(!status.claim_failure(),
            "the marker was consumed, not left behind");
}

#[test]
fn the_failure_notification_names_no_agent_to_select() {
    let response = gpui_kit::SystemNotificationResponse { tag:
                                                              MCP_FAILURE_NOTIFICATION_TAG.into(),
                                                          action_id: None, };

    assert_eq!(notification_response_agent_id(&response),
               None,
               "a click on the MCP failure brings Knot forward without selecting an agent");
}

#[test]
fn the_failure_notification_text_comes_from_the_catalog() {
    for key in ["mcp_server.failure_title", "mcp_server.failure_body"] {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// Polls `condition` until it holds, failing the test rather than hanging.
async fn wait_until(mut condition: impl FnMut() -> bool, what: &str) {
    for _ in 0..500 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    panic!("timed out waiting for {what}");
}
