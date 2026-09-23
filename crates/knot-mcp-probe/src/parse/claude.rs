//! `claude mcp list`.
//!
//! No `--json`: the output is written for people, so the fixture beside this
//! file is the contract and every failure mode degrades rather than throwing
//! the listing away.
//!
//! ```text
//! github: https://api.example/mcp (HTTP) - ✔ Connected
//! aws-mcp: https://aws.example/mcp (HTTP) - ✘ Failed to connect — -32602: …
//! tolaria: https://tolaria.example/mcp (HTTP) - ⊘ Disabled for this project
//! ```

use crate::parse::line::FormatSpec;

/// Claude Code's own status glyphs, which precede nothing here - it puts
/// them after the separator, not at the start of the line - but are stripped
/// defensively in case a future version moves them.
pub(crate) const SPEC: FormatSpec = FormatSpec { leading_glyphs:    &['✔', '✘', '⊘', '⏸'],
                                                 transport_markers: &["(HTTP)", "(SSE)", "(http)",
                                                                      "(sse)", "(stdio)"],
                                                 empty_markers:     &["no mcp servers configured"], };
