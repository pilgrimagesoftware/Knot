//! The periodic vitals line.
//!
//! A silent log is ambiguous: an idle server and a dead one produce exactly
//! the same nothing. A line on a fixed interval removes the ambiguity, and
//! carrying the vitals makes it worth the space - uptime says whether the
//! server restarted, the bound address says where it is answering, the
//! session count says whether anything is connected, and the request count
//! says whether anything is being asked of it.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;

use crate::log::entry::Subject;
use crate::log::logger::Logger;
use crate::session::McpSessionManager;

/// Everything one heartbeat needs to describe the server.
pub(crate) struct Vitals {
    pub(crate) log:      Logger,
    pub(crate) addr:     SocketAddr,
    pub(crate) started:  Instant,
    pub(crate) sessions: McpSessionManager,
    pub(crate) served:   Arc<AtomicU64>,
    pub(crate) interval: Duration,
}

/// Starts the heartbeat, returning its task for the server to own and stop.
pub(crate) fn spawn(vitals: Vitals) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(vitals.interval);
        // The first tick of a tokio interval fires immediately. Consumed
        // here so the log does not open with a heartbeat reporting zero
        // seconds of uptime, which says nothing and is the line most likely
        // to be mistaken for the interval having elapsed.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            vitals.log.info(Subject::Heartbeat, line(&vitals));
        }
    })
}

/// One heartbeat's text, taking the interval's request count as it goes.
fn line(vitals: &Vitals) -> String {
    // `swap` rather than a load then a store: a request landing between the
    // two would be counted in both intervals or in neither.
    let served = vitals.served.swap(0, Ordering::Relaxed);
    let uptime = vitals.started.elapsed().as_secs();
    let sessions = vitals.sessions.len();
    let addr = vitals.addr;
    format!("up {uptime}s on {addr}, {sessions} sessions, {served} requests since last")
}

#[cfg(test)]
mod tests;
