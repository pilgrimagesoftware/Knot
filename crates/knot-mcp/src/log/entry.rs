//! One log entry: what it carries, and how it renders to a JSON line.

use std::fmt;

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// How much attention an entry deserves. Deliberately three values rather
/// than the usual five: this log records what a local server did, and a
/// reader looking for a problem wants `WARN`/`ERROR` to mean something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        };
        f.write_str(text)
    }
}

/// What an entry is about. A closed vocabulary rather than a free string, so
/// a reader can filter the file by subject and a writer cannot invent a
/// fourth spelling of "request".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// The server's own start, bind, and stop.
    Lifecycle,
    /// A JSON-RPC request arriving.
    Request,
    /// A JSON-RPC response leaving.
    Response,
    /// Tool catalog and tool dispatch.
    Tool,
    /// The periodic vitals line.
    Heartbeat,
    /// The log reporting on itself - a write that failed, and its recovery.
    Log,
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Lifecycle => "lifecycle",
            Self::Request => "request",
            Self::Response => "response",
            Self::Tool => "tool",
            Self::Heartbeat => "heartbeat",
            Self::Log => "log",
        };
        f.write_str(text)
    }
}

/// A single entry, timestamped where the event happened rather than where it
/// is written: a backlogged writer must not reorder the record relative to
/// reality.
#[derive(Debug, Clone)]
pub struct Entry {
    pub at:      OffsetDateTime,
    /// The process that logged the entry. Every Knot instance sharing a home
    /// directory appends to the same file, and without this a line cannot be
    /// told apart from one a dev or test build wrote beside it.
    pub pid:     u32,
    pub level:   Level,
    pub subject: Subject,
    pub message: String,
}

impl Entry {
    /// Stamps `message` with the current UTC time.
    pub fn now(level: Level, subject: Subject, message: impl Into<String>) -> Self {
        Self { at: OffsetDateTime::now_utc(),
               pid: std::process::id(),
               level,
               subject,
               message: message.into() }
    }

    /// The entry as it appears in the log file: one JSON object, with no
    /// trailing newline - the writer adds that.
    ///
    /// One entry is always one line. JSON escapes every control character
    /// inside a string, so a message carrying a newline (a path with one in
    /// it, an error whose `Display` spans lines) cannot split into what looks
    /// like two entries and break line-wise reading of the file.
    pub fn render(&self) -> String {
        let time = self.at
                       .format(&Rfc3339)
                       .unwrap_or_else(|_| self.at.unix_timestamp().to_string());
        let line = Line { time:    &time,
                          pid:     self.pid,
                          level:   &self.level.to_string(),
                          subject: &self.subject.to_string(),
                          message: &self.message, };
        // Serializing borrowed strings and an integer cannot fail.
        serde_json::to_string(&line).unwrap_or_default()
    }
}

/// The file's field order and names. A struct rather than a `json!` map, so
/// the order is fixed by declaration and a reader can rely on it.
#[derive(Serialize)]
struct Line<'a> {
    time:    &'a str,
    pid:     u32,
    level:   &'a str,
    subject: &'a str,
    message: &'a str,
}

/// One line of the file, parsed, for tests that assert on what reached disk.
#[cfg(test)]
pub(crate) fn parse_line(line: &str) -> serde_json::Value {
    serde_json::from_str(line).unwrap_or_else(|error| panic!("not a JSON line ({error}): {line}"))
}

#[cfg(test)]
mod tests;
