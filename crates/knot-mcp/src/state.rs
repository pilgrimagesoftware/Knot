//! The MCP server's observable lifecycle state.
//!
//! Implements the observable-lifecycle-state requirement of
//! `openspec/changes/supervise-mcp-server/specs/mcp-server/spec.md`.
//!
//! Each variant carries its own payload rather than the type carrying a
//! union of every field: an observer that reads an address can only have
//! read it from [`ServerState::Running`], so a stale address cannot survive
//! a transition into retrying. That is why the accessors below all answer
//! `None` outside the one variant that owns the data.
//!
//! The error is carried as a `String` rather than as [`crate::McpError`]:
//! the state travels over a `tokio::sync::watch` channel, which requires
//! `Clone`, and every consumer of the error displays it.

use std::net::SocketAddr;
use std::time::Duration;

/// Where the MCP server is in its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ServerState {
    /// Configuration has the server turned off: nothing is bound and
    /// nothing is supervised.
    #[default]
    Disabled,
    /// A bind is in progress.
    Starting,
    /// Bound and serving on `addr`.
    Running { addr: SocketAddr },
    /// The last attempt failed. `attempt` counts attempts made so far,
    /// `next_delay` is the wait before the next one, and `error` is what
    /// the last attempt failed with.
    Retrying {
        attempt:    u32,
        next_delay: Duration,
        error:      String,
    },
    /// The application asked the server to stop.
    Stopped,
}

impl ServerState {
    /// The bound address, which only a running server has.
    pub fn bound_addr(&self) -> Option<SocketAddr> {
        match self {
            Self::Running { addr } => Some(*addr),
            _ => None,
        }
    }

    /// The attempt count, which only a retrying server has.
    pub fn attempt(&self) -> Option<u32> {
        match self {
            Self::Retrying { attempt, .. } => Some(*attempt),
            _ => None,
        }
    }

    /// The wait before the next attempt, which only a retrying server has.
    pub fn next_delay(&self) -> Option<Duration> {
        match self {
            Self::Retrying { next_delay, .. } => Some(*next_delay),
            _ => None,
        }
    }

    /// The last attempt's error, which only a retrying server has.
    pub fn last_error(&self) -> Option<&str> {
        match self {
            Self::Retrying { error, .. } => Some(error.as_str()),
            _ => None,
        }
    }

    /// Whether the server is serving right now.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    /// Whether supervision is currently failing to keep the server up.
    ///
    /// This is the edge the failure notification is raised on, so it names
    /// one state rather than "not running": stopped and disabled are
    /// intentional, and starting has not failed at anything yet.
    pub fn is_failing(&self) -> bool {
        matches!(self, Self::Retrying { .. })
    }
}

#[cfg(test)]
mod tests;
