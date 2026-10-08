//! The tokio runtime a workspace window owns, with a bounded teardown.
//!
//! Dropping a `tokio::runtime::Runtime` waits, with no limit, for every
//! `spawn_blocking` task still running. A window's blocking work - `git`,
//! `ps`, `gh`, an agent CLI's MCP listing - runs under timeouts, but those
//! bound the work, not the wait: anything that slipped past them would make
//! closing the window hang the app, or a test, for as long as it ran. So this
//! shuts down with [`WINDOW_RUNTIME_SHUTDOWN_TIMEOUT`] instead, and a task
//! still running past it is left to finish on its own thread.

use std::ops::Deref;

use tokio::runtime::Runtime;

use crate::consts::WINDOW_RUNTIME_SHUTDOWN_TIMEOUT;

/// A [`Runtime`] whose drop waits at most
/// [`WINDOW_RUNTIME_SHUTDOWN_TIMEOUT`] for its blocking tasks. Derefs to the
/// runtime, so it is used exactly as one.
pub(crate) struct WindowRuntime(Option<Runtime>);

impl WindowRuntime {
    pub(crate) fn new() -> std::io::Result<Self> {
        Runtime::new().map(|runtime| Self(Some(runtime)))
    }
}

impl Deref for WindowRuntime {
    type Target = Runtime;

    fn deref(&self) -> &Runtime {
        // Taken only by `drop`, after which nothing can deref.
        self.0
            .as_ref()
            .expect("the runtime is only taken when the window drops")
    }
}

impl Drop for WindowRuntime {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_timeout(WINDOW_RUNTIME_SHUTDOWN_TIMEOUT);
        }
    }
}

#[cfg(test)]
mod tests;
