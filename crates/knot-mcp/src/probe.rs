//! Liveness probing of a running MCP server.
//!
//! Implements the stopped-answering requirement of
//! `openspec/changes/supervise-mcp-server/specs/mcp-server/spec.md`.
//!
//! A serve task can keep running while the server has stopped answering, so
//! watching the task alone is not enough to tell "serving" from "alive but
//! useless". This asks the server the one question it always answers.
//!
//! The request is written by hand over a `TcpStream` rather than issued
//! through an HTTP client: one unauthenticated GET against loopback does
//! not justify pulling `reqwest`, its hyper stack and its connection pool
//! into the runtime dependencies. It parses one status line from a server
//! whose response shape this crate controls, and it must not grow into a
//! general client - anything more than that belongs to a real client on the
//! dev side, where the tests already use one.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The bytes of a minimal HTTP/1.1 request for the health endpoint.
///
/// `Connection: close` so the server hangs up after answering and the read
/// ends on its own rather than on the timeout.
fn health_request(addr: SocketAddr) -> String {
    format!("GET /health HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n")
}

/// Whether the status line reports success.
///
/// Only `200` counts: the health endpoint has exactly one success answer,
/// and treating any other 2xx as healthy would accept a response this
/// server never sends.
fn is_ok_status_line(line: &str) -> bool {
    let mut parts = line.split_whitespace();
    let version = parts.next().unwrap_or_default();
    let status = parts.next().unwrap_or_default();
    version.starts_with("HTTP/") && status == "200"
}

/// Probes `addr`'s health endpoint, answering whether it reported healthy.
///
/// Connect, write and read together are bounded by `timeout`: a probe that
/// has not finished in that long has failed, whatever stage it is stuck in.
/// Every failure mode - refused connection, reset, timeout, a non-200
/// status - is the same answer, because the caller does the same thing with
/// all of them.
pub async fn probe_health(addr: SocketAddr, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, exchange(addr)).await
                                                 .unwrap_or(false)
}

/// One request/response exchange, unbounded in time; [`probe_health`] is
/// what bounds it.
async fn exchange(addr: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect(addr).await
    else {
        return false;
    };
    if stream.write_all(health_request(addr).as_bytes())
             .await
             .is_err()
    {
        return false;
    }
    read_status_line(&mut stream).await
                                 .is_some_and(|line| is_ok_status_line(&line))
}

/// Reads until the end of the first line, which is as much of the response
/// as the probe has an opinion about.
async fn read_status_line(stream: &mut TcpStream) -> Option<String> {
    let mut received = Vec::new();
    let mut chunk = [0_u8; 128];
    loop {
        let read = stream.read(&mut chunk).await.ok()?;
        if read == 0 {
            // The peer hung up before ending the line.
            return None;
        }
        received.extend_from_slice(&chunk[..read]);
        if let Some(end) = received.iter().position(|byte| *byte == b'\n') {
            let line = String::from_utf8_lossy(&received[..end]);
            return Some(line.trim_end().to_string());
        }
        if received.len() > MAX_STATUS_LINE {
            // Not a status line this server would ever send.
            return None;
        }
    }
}

/// The longest first line the probe will read before giving up on it.
const MAX_STATUS_LINE: usize = 1024;

/// Counts consecutive probe failures against the restart threshold.
///
/// Consecutive, not cumulative: a single dropped connection during a
/// garbage-collection pause says nothing about the server, and bouncing it
/// for one would make supervision the outage. One success clears the count.
#[derive(Debug, Clone)]
pub struct ProbeFailures {
    consecutive: u32,
    threshold:   u32,
}

impl ProbeFailures {
    /// A counter that trips after `threshold` consecutive failures.
    pub fn new(threshold: u32) -> Self {
        Self { consecutive: 0,
               threshold }
    }

    /// How many failures have come in a row.
    pub fn consecutive(&self) -> u32 {
        self.consecutive
    }

    /// Records a failed probe, answering whether the threshold is now met.
    pub fn record_failure(&mut self) -> bool {
        self.consecutive = self.consecutive.saturating_add(1);
        self.is_tripped()
    }

    /// Records a successful probe, clearing the count.
    pub fn record_success(&mut self) {
        self.consecutive = 0;
    }

    /// Whether enough consecutive failures have been recorded to restart.
    pub fn is_tripped(&self) -> bool {
        self.threshold > 0 && self.consecutive >= self.threshold
    }
}

#[cfg(test)]
mod tests;
