//! Keeping the MCP server up.
//!
//! Implements the restart, probe, retry and intentional-stop requirements of
//! `openspec/changes/supervise-mcp-server/specs/mcp-server/spec.md`.
//!
//! The server was started once and never watched again: if its serve task
//! ended, the port went quiet for the rest of the process's life and the
//! only evidence was a line on stderr. This owns that server instead, and
//! owns what to do when it stops working.
//!
//! The loop is one `tokio::select!` over three arms - the stop signal, the
//! serve task's handle, and the probe cycle. That structure is what makes
//! "an intentional stop is never restarted" true by construction rather
//! than by a flag: the stop arm leaves the loop before a task exit can be
//! read as a failure.
//!
//! Every attempt targets the configured port and no other. The MCP URL each
//! agent is launched with is built from that port, so a server that
//! recovered on a different one would be unreachable to every agent already
//! running - which is why a busy port is retried rather than worked around.

use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::{oneshot, watch};
use tokio::task::{AbortHandle, JoinError};
use tokio::time::{Instant, Interval, MissedTickBehavior, interval_at};

use crate::backoff::delay_for;
use crate::consts;
use crate::hooks::AgentHookHandler;
use crate::probe::{ProbeFailures, probe_health};
use crate::server::{AgentsSnapshotFn, McpServer};
use crate::state::ServerState;
use crate::tools::ToolCatalog;

/// The schedule supervision runs on.
///
/// The defaults are the crate's constants; the fields exist so a test can
/// run the same policy on a clock it does not have to wait out. Nothing in
/// the application sets them.
#[derive(Debug, Clone)]
pub struct SupervisorTuning {
    pub backoff_initial_delay:   Duration,
    pub backoff_multiplier:      u32,
    pub backoff_max_delay:       Duration,
    pub probe_interval:          Duration,
    pub probe_timeout:           Duration,
    pub probe_failure_threshold: u32,
}

impl Default for SupervisorTuning {
    fn default() -> Self {
        Self { backoff_initial_delay:   consts::BACKOFF_INITIAL_DELAY,
               backoff_multiplier:      consts::BACKOFF_MULTIPLIER,
               backoff_max_delay:       consts::BACKOFF_MAX_DELAY,
               probe_interval:          consts::PROBE_INTERVAL,
               probe_timeout:           consts::PROBE_TIMEOUT,
               probe_failure_threshold: consts::PROBE_FAILURE_THRESHOLD, }
    }
}

impl SupervisorTuning {
    /// The wait after the `attempt`-th failed attempt on this schedule.
    fn delay(&self, attempt: u32) -> Duration {
        delay_for(attempt,
                  self.backoff_initial_delay,
                  self.backoff_multiplier,
                  self.backoff_max_delay)
    }
}

/// Owns an [`McpServer`] and the policy that keeps one running.
///
/// A fresh server is constructed per attempt from the same catalog, agent
/// snapshot source and hook handler, so a restart serves exactly what the
/// failed one did and no agent has to register again.
pub struct Supervisor {
    port:        u16,
    catalog:     Arc<dyn ToolCatalog>,
    agents:      AgentsSnapshotFn,
    hooks:       Option<Arc<dyn AgentHookHandler>>,
    tuning:      SupervisorTuning,
    state_tx:    watch::Sender<ServerState>,
    /// The abort handle of the serve task currently running.
    ///
    /// Written on every start so the crate's own tests can end a serve task
    /// the way a crash would, which is the one failure the supervisor
    /// reacts to that a test cannot otherwise provoke: the server is owned
    /// inside [`Supervisor::run`] and unreachable from outside it. Nothing
    /// in the application reads this.
    serve_abort: Arc<Mutex<Option<AbortHandle>>>,
}

impl Supervisor {
    /// Supervises `port`, serving `catalog` with `agents` as the status
    /// endpoint's snapshot source.
    ///
    /// The state starts [`ServerState::Disabled`]: nothing is bound and
    /// nothing is supervised until [`Supervisor::run`] is awaited, which is
    /// exactly the state of a server configuration has turned off.
    pub fn new(port: u16, catalog: Arc<dyn ToolCatalog>, agents: AgentsSnapshotFn) -> Self {
        let (state_tx, _) = watch::channel(ServerState::Disabled);
        Self { port,
               catalog,
               agents,
               hooks: None,
               tuning: SupervisorTuning::default(),
               state_tx,
               serve_abort: Arc::new(Mutex::new(None)) }
    }

    pub fn with_hook_handler(mut self, handler: Arc<dyn AgentHookHandler>) -> Self {
        self.hooks = Some(handler);
        self
    }

    pub fn with_tuning(mut self, tuning: SupervisorTuning) -> Self {
        self.tuning = tuning;
        self
    }

    /// The port every attempt binds.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Watches the lifecycle state.
    ///
    /// A `watch` receiver reads the current value on subscription, so a
    /// late subscriber sees where the server is now rather than waiting for
    /// the next transition.
    pub fn state(&self) -> watch::Receiver<ServerState> {
        self.state_tx.subscribe()
    }

    /// Keeps the server up until `stop` fires.
    ///
    /// Returns only after an intentional stop, having released the port.
    /// Every other outcome is retried: a failed bind, a serve task that
    /// ended, and a server that stopped answering its health endpoint are
    /// the same event as far as the policy is concerned.
    pub async fn run(self, mut stop: oneshot::Receiver<()>) {
        let mut attempt: u32 = 0;
        loop {
            self.publish(ServerState::Starting);
            let mut server = self.build_server();
            if let Err(error) = server.start().await {
                attempt = attempt.saturating_add(1);
                if self.wait_before_retry(attempt, error.to_string(), &mut stop)
                       .await
                       .is_break()
                {
                    return;
                }
                continue;
            }

            let Some(addr) = server.bound_addr()
            else {
                // A started server always knows its address; treat the
                // impossible case as a failed attempt rather than trusting
                // a server we cannot probe.
                server.stop();
                attempt = attempt.saturating_add(1);
                let error = "the server bound no address".to_string();
                if self.wait_before_retry(attempt, error, &mut stop)
                       .await
                       .is_break()
                {
                    return;
                }
                continue;
            };

            self.record_serve_abort(&mut server);
            self.publish(ServerState::Running { addr });

            match self.serve(&mut server, addr, &mut attempt, &mut stop).await {
                ServeOutcome::Stopped => {
                    server.stop();
                    self.clear_serve_abort();
                    self.publish(ServerState::Stopped);
                    return;
                }
                ServeOutcome::Failed(error) => {
                    server.stop();
                    drop(server);
                    self.clear_serve_abort();
                    attempt = attempt.saturating_add(1);
                    if self.wait_before_retry(attempt, error, &mut stop)
                           .await
                           .is_break()
                    {
                        return;
                    }
                }
            }
        }
    }

    /// Serves until the application stops the server or the server stops
    /// working.
    ///
    /// `attempt` is reset here, and only here: reaching a bound port is not
    /// evidence of a working server, so the counter clears on a successful
    /// probe rather than on a successful bind. A server that starts and
    /// dies immediately therefore backs off like any other failure instead
    /// of hot-looping.
    async fn serve(&self, server: &mut McpServer, addr: SocketAddr, attempt: &mut u32,
                   stop: &mut oneshot::Receiver<()>)
                   -> ServeOutcome {
        let mut failures = ProbeFailures::new(self.tuning.probe_failure_threshold);
        let mut ticker = self.probe_ticker();
        loop {
            let Some(handle) = server.serve_handle()
            else {
                return ServeOutcome::Failed("the serve task is gone".to_string());
            };
            tokio::select! {
                _ = &mut *stop => return ServeOutcome::Stopped,
                joined = handle => return ServeOutcome::Failed(describe_serve_exit(joined)),
                verdict = probe_cycle(addr,
                                      &mut ticker,
                                      &mut failures,
                                      self.tuning.probe_timeout) => {
                    match verdict {
                        ProbeVerdict::Healthy => *attempt = 0,
                        ProbeVerdict::Degraded => {}
                        ProbeVerdict::Unhealthy => {
                            return ServeOutcome::Failed(
                                format!("the health probe failed {} times in a row",
                                        failures.consecutive()));
                        }
                    }
                }
            }
        }
    }

    /// Publishes the retrying state and waits out its delay, answering
    /// whether the wait was interrupted by an intentional stop.
    async fn wait_before_retry(&self, attempt: u32, error: String,
                               stop: &mut oneshot::Receiver<()>)
                               -> ControlFlow<()> {
        let next_delay = self.tuning.delay(attempt);
        self.publish(ServerState::Retrying { attempt,
                                             next_delay,
                                             error });
        tokio::select! {
            _ = &mut *stop => {
                self.publish(ServerState::Stopped);
                ControlFlow::Break(())
            }
            _ = tokio::time::sleep(next_delay) => ControlFlow::Continue(()),
        }
    }

    fn build_server(&self) -> McpServer {
        let server = McpServer::new(self.port,
                                    Arc::clone(&self.catalog),
                                    Arc::clone(&self.agents));
        match &self.hooks {
            Some(handler) => server.with_hook_handler(Arc::clone(handler)),
            None => server,
        }
    }

    /// The probe timer, first firing one interval after the server came up
    /// rather than immediately - a server that has just bound has not had a
    /// chance to answer anything yet.
    fn probe_ticker(&self) -> Interval {
        let mut ticker = interval_at(Instant::now() + self.tuning.probe_interval,
                                     self.tuning.probe_interval);
        // A probe that overran its interval should not be followed by a
        // burst of catch-up probes.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        ticker
    }

    fn publish(&self, state: ServerState) {
        // `send_replace` rather than `send`: the state is worth recording
        // even with nothing watching yet, and a late subscriber reads it.
        self.state_tx.send_replace(state);
    }

    fn record_serve_abort(&self, server: &mut McpServer) {
        let abort = server.serve_handle().map(|handle| handle.abort_handle());
        *self.serve_abort.lock() = abort;
    }

    fn clear_serve_abort(&self) {
        *self.serve_abort.lock() = None;
    }

    /// A handle that outlives [`Supervisor::run`] - which consumes the
    /// supervisor - and can end the running serve task the way a crash
    /// would. The crate's tests are the only caller; see
    /// [`Supervisor::serve_abort`].
    #[cfg(test)]
    pub(crate) fn serve_aborter(&self) -> ServeAborter {
        ServeAborter(Arc::clone(&self.serve_abort))
    }
}

/// Ends the serve task a supervisor is currently watching.
#[cfg(test)]
pub(crate) struct ServeAborter(Arc<Mutex<Option<AbortHandle>>>);

#[cfg(test)]
impl ServeAborter {
    /// Aborts the running serve task, answering whether there was one.
    pub(crate) fn abort(&self) -> bool {
        match self.0.lock().as_ref() {
            Some(handle) => {
                handle.abort();
                true
            }
            None => false,
        }
    }
}

/// Why [`Supervisor::serve`] returned.
enum ServeOutcome {
    /// The application asked the server to stop.
    Stopped,
    /// The server stopped working, described for the state row.
    Failed(String),
}

/// What a probe cycle concluded.
#[derive(Debug, PartialEq, Eq)]
enum ProbeVerdict {
    /// The server answered.
    Healthy,
    /// The probe failed, but not enough times in a row to act on.
    Degraded,
    /// Enough consecutive probes have failed to restart the server.
    Unhealthy,
}

/// Waits for the next probe tick, probes, and records the result.
///
/// Written as one future so the whole cycle sits in a `select!` arm and an
/// intentional stop is never made to wait out a probe's timeout. Abandoning
/// it mid-probe costs at most one tick, and only ever happens on the way out
/// of the loop.
async fn probe_cycle(addr: SocketAddr, ticker: &mut Interval, failures: &mut ProbeFailures,
                     timeout: Duration)
                     -> ProbeVerdict {
    ticker.tick().await;
    if probe_health(addr, timeout).await {
        failures.record_success();
        return ProbeVerdict::Healthy;
    }
    if failures.record_failure() {
        ProbeVerdict::Unhealthy
    }
    else {
        ProbeVerdict::Degraded
    }
}

/// Describes how the serve task ended, for the state row.
///
/// Every ending is a failure here. The one ending that is not - the
/// application asking the server to stop - never reaches this, because the
/// stop arm of the loop wins first.
fn describe_serve_exit(joined: Result<(), JoinError>) -> String {
    match joined {
        Ok(()) => "the serve task ended".to_string(),
        Err(error) if error.is_panic() => "the serve task panicked".to_string(),
        Err(error) if error.is_cancelled() => "the serve task was cancelled".to_string(),
        Err(error) => format!("the serve task failed: {error}"),
    }
}

#[cfg(test)]
mod tests;
