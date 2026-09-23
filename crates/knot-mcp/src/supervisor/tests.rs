//! Unit tests for [`super`].
//!
//! Timings come from a [`SupervisorTuning`] rather than the crate's
//! constants: the policy under test is the same one, run on a clock a test
//! does not have to wait out.

use std::time::Duration as StdDuration;

use tokio::net::TcpListener;

use super::*;
use crate::tools::EmptyCatalog;

/// How long a test waits for a state it expects. Generous: it bounds a
/// hang, it does not pace the assertion.
const WAIT: StdDuration = StdDuration::from_secs(10);

fn no_agents() -> AgentsSnapshotFn {
    Arc::new(Vec::new)
}

/// Supervision on a short clock, with probing effectively off so a test
/// that is not about probing is not disturbed by one.
fn fast_tuning() -> SupervisorTuning {
    SupervisorTuning { backoff_initial_delay:   StdDuration::from_millis(20),
                       backoff_multiplier:      2,
                       backoff_max_delay:       StdDuration::from_millis(200),
                       probe_interval:          StdDuration::from_secs(3_600),
                       probe_timeout:           StdDuration::from_millis(200),
                       probe_failure_threshold: 3, }
}

fn supervisor(port: u16) -> Supervisor {
    Supervisor::new(port, Arc::new(EmptyCatalog), no_agents()).with_tuning(fast_tuning())
}

/// Takes an ephemeral port and hands back both the listener holding it and
/// its number, so a test can decide when to release it.
async fn held_port() -> (TcpListener, u16) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await
                                                      .expect("an ephemeral port");
    let port = listener.local_addr().expect("bound").port();
    (listener, port)
}

/// A port nothing holds any more.
async fn free_port() -> u16 {
    let (listener, port) = held_port().await;
    drop(listener);
    port
}

/// Waits for a state satisfying `predicate`, failing the test rather than
/// hanging if it never arrives.
async fn wait_for(states: &mut watch::Receiver<ServerState>,
                  predicate: impl FnMut(&ServerState) -> bool, what: &str)
                  -> ServerState {
    let found = tokio::time::timeout(WAIT, states.wait_for(predicate)).await;
    match found {
        Ok(Ok(state)) => state.clone(),
        Ok(Err(error)) => panic!("the supervisor stopped publishing while waiting for {what}: \
                                  {error}"),
        Err(_) => panic!("timed out waiting for {what}"),
    }
}

async fn wait_for_running(states: &mut watch::Receiver<ServerState>) -> SocketAddr {
    wait_for(states, ServerState::is_running, "running").await
        .bound_addr()
        .expect("a running state carries its address")
}

async fn wait_for_retrying(states: &mut watch::Receiver<ServerState>) -> ServerState {
    wait_for(states, ServerState::is_failing, "retrying").await
}

async fn health_of(addr: SocketAddr) -> bool {
    probe_health(addr, StdDuration::from_secs(2)).await
}

#[tokio::test]
async fn a_successful_bind_reaches_running_with_the_bound_address() {
    let port = free_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();
    assert_eq!(*states.borrow(),
               ServerState::Disabled,
               "nothing is supervised before run");

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    let addr = wait_for_running(&mut states).await;
    assert_eq!(addr.port(), port);
    assert!(health_of(addr).await, "the running server answers");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

#[tokio::test]
async fn a_late_subscriber_immediately_sees_running() {
    let port = free_port().await;
    let supervisor = supervisor(port);
    let mut early = supervisor.state();
    let state_source = supervisor.state();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));
    wait_for_running(&mut early).await;

    // Subscribing only now, after the transition into running has already
    // happened.
    let late = state_source.clone();
    assert!(late.borrow().is_running(),
            "a late subscriber reads the current state, not the next");
    assert_eq!(late.borrow().bound_addr().map(|addr| addr.port()),
               Some(port));

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

#[tokio::test]
async fn an_ended_serve_task_is_restarted_on_the_same_port() {
    let port = free_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();
    let aborter = supervisor.serve_aborter();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    let first = wait_for_running(&mut states).await;
    assert!(aborter.abort(), "there is a serve task to end");

    let retrying = wait_for_retrying(&mut states).await;
    assert_eq!(retrying.attempt(), Some(1));

    let second = wait_for_running(&mut states).await;
    assert_eq!(second.port(), port, "a restart never moves to another port");
    assert_eq!(second, first,
               "the restarted server is reachable at the same address");
    assert!(health_of(second).await, "the restarted server answers");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

#[tokio::test]
async fn a_restart_serves_the_same_tools() {
    let port = free_port().await;
    let supervisor =
        Supervisor::new(port, Arc::new(NamedCatalog), no_agents()).with_tuning(fast_tuning());
    let mut states = supervisor.state();
    let aborter = supervisor.serve_aborter();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    let before = wait_for_running(&mut states).await;
    let listed_before = tools_listed(before).await;
    assert_eq!(listed_before,
               vec![NamedCatalog::TOOL],
               "the catalog is served before the failure");

    aborter.abort();
    wait_for_retrying(&mut states).await;
    let after = wait_for_running(&mut states).await;

    assert_eq!(tools_listed(after).await,
               listed_before,
               "the restarted server serves the catalog it was constructed with");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

/// A catalog with one recognizable tool, so "the same tools" is an
/// assertion about a name rather than about two empty lists.
struct NamedCatalog;

impl NamedCatalog {
    const TOOL: &'static str = "knot_supervisor_probe_tool";
}

#[async_trait::async_trait]
impl ToolCatalog for NamedCatalog {
    fn list(&self) -> Vec<crate::ToolDefinition> {
        vec![crate::ToolDefinition { name:         Self::TOOL.to_string(),
                                     description:
                                         "a tool only this test's catalog has".to_string(),
                                     input_schema: crate::ToolInputSchema::default(), }]
    }

    async fn call(&self, name: &str, _arguments: serde_json::Value) -> crate::ToolCallResult {
        crate::ToolCallResult::ok(name)
    }
}

/// The tool names the server at `addr` answers `tools/list` with, asked the
/// way an agent asks.
async fn tools_listed(addr: SocketAddr) -> Vec<String> {
    let body = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" });
    let response: serde_json::Value = reqwest::Client::new().post(format!("http://{addr}/mcp"))
                                                            .json(&body)
                                                            .send()
                                                            .await
                                                            .expect("the server answers /mcp")
                                                            .json()
                                                            .await
                                                            .expect("a JSON-RPC response");
    response["result"]["tools"].as_array()
                               .expect("tools/list returns a list")
                               .iter()
                               .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
                               .collect()
}

#[tokio::test]
async fn a_busy_port_is_retried_with_growing_delays_and_recovers_when_released() {
    let (holder, port) = held_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    let first = wait_for_retrying(&mut states).await;
    assert_eq!(first.attempt(), Some(1));
    assert!(first.last_error().is_some_and(|error| !error.is_empty()),
            "a retrying state names what failed");

    let second = wait_for(&mut states,
                          |state| state.attempt().is_some_and(|attempt| attempt >= 2),
                          "a second attempt").await;
    assert!(second.next_delay() > first.next_delay(),
            "the delay grows: {:?} then {:?}",
            first.next_delay(),
            second.next_delay());

    drop(holder);

    let addr = wait_for_running(&mut states).await;
    assert_eq!(addr.port(),
               port,
               "recovery binds the configured port, never another");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

#[tokio::test]
async fn the_delay_never_grows_past_the_configured_maximum() {
    let tuning = fast_tuning();
    let (holder, port) = held_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    let late = wait_for(&mut states,
                        |state| state.attempt().is_some_and(|attempt| attempt >= 6),
                        "a sixth attempt").await;
    assert_eq!(late.next_delay(),
               Some(tuning.backoff_max_delay),
               "the delay caps");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
    drop(holder);
}

#[tokio::test]
async fn an_intentional_stop_yields_stopped_and_releases_the_port() {
    let port = free_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));
    let addr = wait_for_running(&mut states).await;

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");

    assert_eq!(*states.borrow(),
               ServerState::Stopped,
               "an intentional stop is not a failure");
    assert!(!health_of(addr).await, "the stopped server answers nothing");
    // Rebinding proves the port was released rather than left in the
    // supervisor's hands.
    TcpListener::bind(("127.0.0.1", port)).await
                                          .expect("the configured port is free again");
}

#[tokio::test]
async fn stopping_while_retrying_does_not_start_the_server() {
    let (holder, port) = held_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));
    wait_for_retrying(&mut states).await;

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");

    assert_eq!(*states.borrow(), ServerState::Stopped);
    drop(holder);
    // Nothing of the supervisor's is left holding the port.
    TcpListener::bind(("127.0.0.1", port)).await
                                          .expect("the configured port is free");
}

#[tokio::test]
async fn a_start_then_die_loop_backs_off_rather_than_hot_looping() {
    let port = free_port().await;
    let supervisor = supervisor(port);
    let mut states = supervisor.state();
    let aborter = supervisor.serve_aborter();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    // Kill the server as soon as it comes up, three episodes running. The
    // probe interval is an hour here, so no probe ever succeeds to reset
    // the counter - a successful bind alone must not.
    let mut attempts = Vec::new();
    for _ in 0..3 {
        wait_for_running(&mut states).await;
        aborter.abort();
        let retrying = wait_for(&mut states,
                                |state| {
                                    state.attempt()
                                         .is_some_and(|attempt| {
                                             attempts.last().is_none_or(|last| attempt > *last)
                                         })
                                },
                                "a further retry").await;
        attempts.push(retrying.attempt()
                              .expect("a retrying state counts attempts"));
    }

    assert_eq!(attempts,
               vec![1, 2, 3],
               "the attempt counter survives a successful bind");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

#[tokio::test]
async fn a_probe_that_succeeds_resets_the_attempt_counter() {
    let port = free_port().await;
    let tuning = SupervisorTuning { probe_interval: StdDuration::from_millis(20),
                                    ..fast_tuning() };
    let supervisor = Supervisor::new(port, Arc::new(EmptyCatalog), no_agents()).with_tuning(tuning);
    let mut states = supervisor.state();
    let aborter = supervisor.serve_aborter();

    let (stop, stop_rx) = oneshot::channel();
    let supervising = tokio::spawn(supervisor.run(stop_rx));

    wait_for_running(&mut states).await;
    aborter.abort();
    assert_eq!(wait_for_retrying(&mut states).await.attempt(), Some(1));

    // Let the restarted server pass a probe, then kill it again: the count
    // starts over because a probe proved it was working.
    wait_for_running(&mut states).await;
    tokio::time::sleep(StdDuration::from_millis(120)).await;
    aborter.abort();
    assert_eq!(wait_for_retrying(&mut states).await.attempt(),
               Some(1),
               "a healthy probe clears the backoff");

    let _ = stop.send(());
    supervising.await.expect("supervision ends cleanly");
}

#[tokio::test]
async fn the_probe_cycle_trips_only_after_consecutive_failures() {
    let addr: SocketAddr = ([127, 0, 0, 1], free_port().await).into();
    let mut failures = ProbeFailures::new(2);
    let mut ticker = interval_at(Instant::now(), StdDuration::from_millis(5));
    let timeout = StdDuration::from_millis(200);

    assert_eq!(probe_cycle(addr, &mut ticker, &mut failures, timeout).await,
               ProbeVerdict::Degraded,
               "one failed probe is tolerated");
    assert_eq!(probe_cycle(addr, &mut ticker, &mut failures, timeout).await,
               ProbeVerdict::Unhealthy,
               "the threshold restarts the server");
}

#[tokio::test]
async fn the_probe_cycle_reports_a_live_server_healthy() {
    let mut server = McpServer::new(0, Arc::new(EmptyCatalog), no_agents());
    server.start().await.expect("server starts");
    let addr = server.bound_addr().expect("bound");
    let mut failures = ProbeFailures::new(2);
    failures.record_failure();
    let mut ticker = interval_at(Instant::now(), StdDuration::from_millis(5));

    let verdict = probe_cycle(addr, &mut ticker, &mut failures, StdDuration::from_secs(2)).await;

    assert_eq!(verdict, ProbeVerdict::Healthy);
    assert_eq!(failures.consecutive(),
               0,
               "a healthy probe clears the count");
    server.stop();
}

#[tokio::test]
async fn a_panicking_serve_task_is_described_as_a_failure() {
    let panicked = tokio::spawn(async { panic!("the serve task fell over") }).await;
    assert_eq!(describe_serve_exit(panicked), "the serve task panicked");
}

#[tokio::test]
async fn a_returning_serve_task_is_described_as_a_failure() {
    let returned = tokio::spawn(async {}).await;
    assert_eq!(describe_serve_exit(returned), "the serve task ended");
}

#[tokio::test]
async fn an_aborted_serve_task_is_described_as_a_failure() {
    let handle = tokio::spawn(std::future::pending::<()>());
    handle.abort();
    let aborted = handle.await;
    assert_eq!(describe_serve_exit(aborted), "the serve task was cancelled");
}

#[test]
fn the_default_tuning_is_the_crate_constants() {
    let tuning = SupervisorTuning::default();
    assert_eq!(tuning.backoff_initial_delay, consts::BACKOFF_INITIAL_DELAY);
    assert_eq!(tuning.backoff_multiplier, consts::BACKOFF_MULTIPLIER);
    assert_eq!(tuning.backoff_max_delay, consts::BACKOFF_MAX_DELAY);
    assert_eq!(tuning.probe_interval, consts::PROBE_INTERVAL);
    assert_eq!(tuning.probe_timeout, consts::PROBE_TIMEOUT);
    assert_eq!(tuning.probe_failure_threshold,
               consts::PROBE_FAILURE_THRESHOLD);
}
