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

use crate::error::{ProbeError, Result};
use crate::server::ServerRow;

/// How to read one agent type's MCP listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    /// `claude mcp list`: `<name>: <target> - <glyph> <state>`, no `--json`.
    ClaudeCode,
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
        let rows = match self {
            Self::ClaudeCode => claude::parse(output),
        };

        rows.ok_or_else(|| ProbeError::Unrecognized { program:    program.to_owned(),
                                                      first_line: first_meaningful_line(output), })
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
