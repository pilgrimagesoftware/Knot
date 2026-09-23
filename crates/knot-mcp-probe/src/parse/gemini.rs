//! `gemini mcp list`.
//!
//! The same grammar as Claude Code's with two differences: the status glyph
//! leads the line rather than trailing it, and the state is a plain word with
//! no glyph at all.
//!
//! ```text
//! ○ tolaria: /opt/homebrew/bin/node /Applications/…/index.js (stdio) - Disabled
//! ● github: https://api.example/mcp (http) - Connected
//! ```
//!
//! Gemini also prefixes an untrusted folder's listing with a `Warning: …`
//! line. It has the `name: value` shape, which is why an entry is required to
//! carry the state separator too - otherwise the warning becomes a server
//! called "Warning".

use crate::parse::line::FormatSpec;

pub(crate) const SPEC: FormatSpec =
    FormatSpec { leading_glyphs:    &['○', '●', '✓', '✗', '•'],
                 transport_markers: &["(stdio)", "(http)", "(sse)", "(HTTP)", "(SSE)"],
                 empty_markers:     &["no mcp servers configured", "no configured mcp servers"], };

#[cfg(test)]
mod tests;
