//! One MCP server as the section shows it: what it is called, how it is
//! reached, and what state it is in.
//!
//! The target is kept whole and shown short. A stdio server's command line is
//! unbounded - the entry that prompted [`consts::MAX_LABEL_CHARS`] was a
//! 1.5 KB `node -e` program - and the row that renders it sits in a
//! virtualized list, where a row taller than it declared corrupts the scroll
//! position of everything below it. So [`ServerRow::short_label`] is what a
//! row draws and [`Target::full`] is what a copy action hands over.

use crate::consts::MAX_LABEL_CHARS;
use crate::state::ServerState;

/// How a server is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// An HTTP or SSE endpoint.
    Http { url: String },
    /// A command the agent runs and speaks to over its pipes.
    Stdio { command: String },
}

impl Target {
    /// The transport's stable token.
    #[must_use]
    pub fn token(&self) -> &'static str {
        match self {
            Self::Http { .. } => "http",
            Self::Stdio { .. } => "stdio",
        }
    }

    /// The complete URL or command line, for the copy action.
    #[must_use]
    pub fn full(&self) -> &str {
        match self {
            Self::Http { url } => url,
            Self::Stdio { command } => command,
        }
    }

    /// A bounded identifier: the host for HTTP, the program's basename for
    /// stdio, truncated either way.
    #[must_use]
    pub fn short_label(&self) -> String {
        let bounded = match self {
            Self::Http { url } => host_of(url),
            Self::Stdio { command } => program_basename(command),
        };

        truncate(bounded, MAX_LABEL_CHARS)
    }
}

/// One server, as one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerRow {
    /// What the agent's configuration calls it. Not assumed unique and not
    /// assumed shell-safe: it comes from foreign output.
    pub name:   String,
    pub target: Target,
    pub state:  ServerState,
    /// Why, when the tooling said. Carried beside the state rather than
    /// inside it so the vocabulary stays closed.
    pub detail: Option<String>,
}

impl ServerRow {
    /// A row in a state, with no detail.
    #[must_use]
    pub fn new(name: impl Into<String>, target: Target, state: ServerState) -> Self {
        Self { name: name.into(),
               target,
               state,
               detail: None }
    }

    /// The same row, carrying the reason the tooling gave.
    #[must_use]
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        self.detail = (!detail.trim().is_empty()).then_some(detail);
        self
    }

    /// What the row draws for its target.
    #[must_use]
    pub fn short_label(&self) -> String {
        self.target.short_label()
    }
}

/// The host (with port) of a URL, without pulling in a URL parser.
///
/// Everything this needs to survive is malformed input: the string comes from
/// another program's output, so "not a URL at all" is an ordinary case and
/// returning it whole - bounded by the caller - beats guessing.
fn host_of(url: &str) -> &str {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = after_scheme.split(['/', '?', '#'])
                                .next()
                                .unwrap_or(after_scheme);

    // Userinfo would be a credential; a row must not draw one.
    authority.rsplit_once('@')
             .map_or(authority, |(_, host)| host)
}

/// The basename of the program a command line runs.
///
/// The first whitespace-separated token, less its directories. Arguments are
/// dropped deliberately: they are where the unbounded length lives, and they
/// do not identify the server.
fn program_basename(command: &str) -> &str {
    let program = command.split_whitespace().next().unwrap_or("");

    program.rsplit('/').next().unwrap_or(program)
}

/// Truncates on a character boundary, marking that it did.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }

    let kept: String = text.chars().take(max.saturating_sub(1)).collect();

    format!("{kept}…")
}

#[cfg(test)]
mod tests;
