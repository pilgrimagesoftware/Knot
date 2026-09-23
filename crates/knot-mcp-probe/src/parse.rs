//! Turning an agent CLI's listing into rows.
//!
//! One format per agent type that has one. The formats live here rather than
//! on `knot_core::agent_type`'s roster because parsing is this crate's
//! business and `knot-core` must not depend on it - the same split
//! `knot-agent-launch` makes for ACP adapters and `knot-history` for
//! transcript readers. What keeps that from becoming a second, drifting
//! roster is the coverage test that fails when a known agent type has a list
//! command here and no format, or the reverse.

pub(crate) mod claude;
pub(crate) mod gemini;
pub(crate) mod line;

use crate::error::{ProbeError, Result};
use crate::server::ServerRow;

/// How to read one agent type's MCP listing.
///
/// A variant exists only for a format whose real output has been captured.
/// An agent type with no variant reports "cannot determine" rather than
/// getting a reader written from documentation - a format inferred rather
/// than observed fails silently, which is the one failure mode this crate is
/// built to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    /// `claude mcp list`: `<name>: <target> - <glyph> <state>`, no `--json`.
    ClaudeCode,
    /// `gemini mcp list`: `<glyph> <name>: <target> (<transport>) - <state>`.
    Gemini,
}

impl ListFormat {
    /// Reads a listing.
    ///
    /// # Errors
    ///
    /// [`ProbeError::Unrecognized`] when no line in `output` looks like a
    /// server. That is deliberately not an empty list: output the parser
    /// cannot read means the format moved, and reporting it as "no servers
    /// configured" would be a confident wrong answer where the honest one is
    /// "I could not read this".
    pub fn read(self, program: &str, output: &str) -> Result<Vec<ServerRow>> {
        let rows = line::parse(output, self.spec());

        rows.ok_or_else(|| ProbeError::Unrecognized { program:    program.to_owned(),
                                                      first_line: first_meaningful_line(output), })
    }

    /// The format for an agent type, or `None` when Knot has no reader for
    /// it.
    ///
    /// This table is the per-type data that genuinely belongs to this crate -
    /// `knot-core` cannot name a [`ListFormat`] without depending on the
    /// parser, the same way it cannot name an ACP adapter or a transcript
    /// reader. What stops it becoming a second, drifting roster is the
    /// coverage test that fails when a type has a list command and no format,
    /// or a format and no list command.
    #[must_use]
    pub fn for_agent_type(id: &str) -> Option<Self> {
        match id {
            "claude" => Some(Self::ClaudeCode),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }

    fn spec(self) -> &'static line::FormatSpec {
        match self {
            Self::ClaudeCode => &claude::SPEC,
            Self::Gemini => &gemini::SPEC,
        }
    }
}

/// The line to quote when the output could not be read.
///
/// Prefers a line that at least looks like an entry over the first line
/// outright: `claude mcp list` opens with a progress header, and quoting
/// "Checking MCP server health…" would name the one line that was never the
/// problem.
fn first_meaningful_line(output: &str) -> String {
    let mut lines = output.lines()
                          .map(str::trim)
                          .filter(|line| !line.is_empty());

    let first = lines.clone().next().unwrap_or_default();

    lines.find(|line| line.contains(": "))
         .unwrap_or(first)
         .to_owned()
}

#[cfg(test)]
mod tests;
