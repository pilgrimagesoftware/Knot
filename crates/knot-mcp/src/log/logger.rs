//! The handle every call site holds.

use std::path::PathBuf;

use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;

use crate::log::entry::{Entry, Level, Subject};
use crate::log::writer::Writer;

/// A cheap, clonable handle to the log.
///
/// Logging is a channel send: it does not block, does not lock, and does not
/// touch the filesystem, so it is safe on the request path. The channel is
/// unbounded deliberately - a bounded one would have to choose between
/// blocking a request handler and silently dropping entries, and at a
/// handful of lines per request neither is worth buying. A writer that falls
/// behind grows memory until it catches up, which for a local diagnostics
/// log is the right failure.
#[derive(Clone)]
pub struct Logger {
    entries: UnboundedSender<Entry>,
}

impl Logger {
    /// Starts the writer task for the log file at `path` and returns the
    /// handle to send through, plus the task's join handle for the caller to
    /// own.
    ///
    /// The task is not detached: the caller stores the handle and aborts it,
    /// so a spawned task cannot outlive the server that started it or
    /// swallow its own panic.
    pub fn spawn(path: PathBuf) -> (Self, JoinHandle<()>) {
        let (entries, receiver) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move { Writer::open(path).run(receiver).await });
        (Self { entries }, task)
    }

    /// Records `message`, stamped with the moment of this call.
    ///
    /// A send that fails means the writer task is gone, which happens only
    /// during shutdown. There is nowhere useful to report that - the log is
    /// the thing that has stopped - so it is dropped.
    pub fn log(&self, level: Level, subject: Subject, message: impl Into<String>) {
        let _ = self.entries.send(Entry::now(level, subject, message));
    }

    /// Records an ordinary event.
    pub fn info(&self, subject: Subject, message: impl Into<String>) {
        self.log(Level::Info, subject, message);
    }

    /// Records something that went wrong but did not stop the server.
    pub fn warn(&self, subject: Subject, message: impl Into<String>) {
        self.log(Level::Warn, subject, message);
    }

    /// Records a failure.
    pub fn error(&self, subject: Subject, message: impl Into<String>) {
        self.log(Level::Error, subject, message);
    }
}

#[cfg(test)]
mod tests;
