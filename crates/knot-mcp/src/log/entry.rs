//! One log entry: what it carries, and how it renders to a line.

use std::fmt;

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
    pub level:   Level,
    pub subject: Subject,
    pub message: String,
}

impl Entry {
    /// Stamps `message` with the current UTC time.
    pub fn now(level: Level, subject: Subject, message: impl Into<String>) -> Self {
        Self { at: OffsetDateTime::now_utc(),
               level,
               subject,
               message: message.into() }
    }

    /// The entry as it appears in the file and on standard error, with no
    /// trailing newline - the writer adds that.
    ///
    /// One entry is always one line. A message carrying a newline (a path
    /// with one in it, an error whose `Display` spans lines) would otherwise
    /// split into what looks like two entries, the second with no timestamp,
    /// and break every line-wise search of the file.
    pub fn render(&self) -> String {
        let at = self.at
                     .format(&Rfc3339)
                     .unwrap_or_else(|_| self.at.unix_timestamp().to_string());
        format!("{at} {} {} {}",
                self.level,
                self.subject,
                escape(&self.message))
    }
}

/// Replaces the characters that would end a line, or that would be read as an
/// escape of one, with their two-character escapes.
fn escape(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    for character in message.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests;
