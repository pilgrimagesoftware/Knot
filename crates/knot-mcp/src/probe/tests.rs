//! Unit tests for [`super`].

use std::sync::Arc;
use std::time::Instant;

use super::*;
use crate::consts::PROBE_FAILURE_THRESHOLD;
use crate::server::{AgentsSnapshotFn, McpServer};
use crate::tools::EmptyCatalog;

fn no_agents() -> AgentsSnapshotFn {
    Arc::new(Vec::new)
}

/// A port nothing is listening on: bind it, learn its number, drop the
/// listener. Racing another process onto it in the gap is possible in
/// principle and has no bearing on the assertions, which only require that
/// the probe answers within its timeout.
async fn closed_port() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await
                                                                  .expect("an ephemeral port");
    let addr = listener.local_addr().expect("bound");
    drop(listener);
    addr
}

#[tokio::test]
async fn a_live_server_probes_healthy() {
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents());
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound after start");

    assert!(probe_health(addr, Duration::from_secs(5)).await);

    server.stop();
}

#[tokio::test]
async fn a_stopped_server_probes_unhealthy() {
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents());
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound after start");
    server.stop();

    assert!(!probe_health(addr, Duration::from_secs(1)).await);
}

#[tokio::test]
async fn a_closed_port_fails_within_the_timeout_rather_than_hanging() {
    let addr = closed_port().await;
    let timeout = Duration::from_millis(500);

    let started = Instant::now();
    let healthy = probe_health(addr, timeout).await;
    let elapsed = started.elapsed();

    assert!(!healthy, "a closed port is not healthy");
    assert!(elapsed < timeout * 4,
            "the probe outran its own timeout: {elapsed:?}");
}

#[test]
fn only_a_200_status_line_is_healthy() {
    assert!(is_ok_status_line("HTTP/1.1 200 OK"));
    assert!(is_ok_status_line("HTTP/1.0 200"));
    assert!(!is_ok_status_line("HTTP/1.1 204 No Content"));
    assert!(!is_ok_status_line("HTTP/1.1 500 Internal Server Error"));
    assert!(!is_ok_status_line("200 OK"));
    assert!(!is_ok_status_line(""));
}

#[test]
fn the_request_names_the_health_endpoint_and_asks_to_close() {
    let request = health_request(([127, 0, 0, 1], 8767).into());
    assert!(request.starts_with("GET /health HTTP/1.1\r\n"));
    assert!(request.contains("Connection: close\r\n"));
    assert!(request.ends_with("\r\n\r\n"));
}

#[test]
fn one_failure_does_not_trip_the_counter() {
    let mut failures = ProbeFailures::new(PROBE_FAILURE_THRESHOLD);
    assert!(!failures.record_failure());
    assert_eq!(failures.consecutive(), 1);
    assert!(!failures.is_tripped());
}

#[test]
fn the_configured_number_of_consecutive_failures_trips_the_counter() {
    let mut failures = ProbeFailures::new(3);
    assert!(!failures.record_failure());
    assert!(!failures.record_failure());
    assert!(failures.record_failure(),
            "the third consecutive failure trips");
    assert!(failures.is_tripped());
}

#[test]
fn one_success_resets_the_count() {
    let mut failures = ProbeFailures::new(3);
    failures.record_failure();
    failures.record_failure();
    failures.record_success();

    assert_eq!(failures.consecutive(), 0);
    assert!(!failures.is_tripped());
    assert!(!failures.record_failure(),
            "counting restarts from zero after a success");
}

#[test]
fn the_count_saturates_rather_than_wrapping() {
    let mut failures = ProbeFailures::new(3);
    for _ in 0..10 {
        failures.record_failure();
    }
    assert_eq!(failures.consecutive(), 10);
    assert!(failures.is_tripped());
}
