//! The per-tool subagent-definition provider trait and the registry that
//! resolves one by coding-agent tool.
//!
//! Mirrors `knot-history`'s history-provider registry deliberately: one trait,
//! one `provider()` returning `None` for anything unsupported, one
//! `supports_*` built on it. A reader who knows that one knows this one.
//!
//! Contract: `openspec/specs/data-import/spec.md` - "Subagent definition
//! provider registry".

mod claude;
#[cfg(test)]
mod tests;

use std::fmt;
use std::path::Path;
use std::str::FromStr;

pub use claude::ClaudeSubagentProvider;

use crate::import::result::Unreadable;

/// A coding-agent tool that can hold subagent definitions.
///
/// A closed vocabulary, so an enum rather than a matched `String`: every site
/// that handles a tool is exhaustive, and the compiler finds the next one when
/// a provider lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubagentTool {
    Claude,
    Codex,
    OpenCode,
    Gemini,
}

impl SubagentTool {
    /// Every tool the registry knows of, implemented or not.
    pub const ALL: [Self; 4] = [Self::Claude, Self::Codex, Self::OpenCode, Self::Gemini];

    /// The tools with a reader behind them, in listing order.
    ///
    /// The Import tab lists these rather than [`Self::ALL`]: a tool whose
    /// format has not been read from a real installation would report
    /// "nothing to import", which is indistinguishable from a user who has no
    /// definitions. Absent is honest; falsely empty is not.
    #[must_use]
    pub fn implemented() -> Vec<Self> {
        Self::ALL.into_iter()
                 .filter(|tool| supports_subagent_import(*tool))
                 .collect()
    }

    /// The tool's display name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::OpenCode => "OpenCode",
            Self::Gemini => "Gemini",
        }
    }
}

impl fmt::Display for SubagentTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Gemini => "gemini",
        };
        f.write_str(name)
    }
}

impl FromStr for SubagentTool {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            "opencode" => Ok(Self::OpenCode),
            "gemini" => Ok(Self::Gemini),
            _ => Err(()),
        }
    }
}

/// One subagent definition as its source tool holds it, before it becomes a
/// persona.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentDefinition {
    /// The definition's own name, from the source. Never the file name.
    pub name:         String,
    /// The system prompt, which is what a Knot persona's instructions are.
    pub instructions: String,
    /// Where it was read from, shown so the user can tell a project-level
    /// definition from a user-level one of the same name.
    pub source:       String,
}

/// Everything one scan of a tool's definitions turned up: the ones that could
/// be read and the ones that could not.
///
/// The two travel together because `data-import` forbids one malformed file
/// from abandoning the rest - the reader returns both halves and lets the
/// caller import the good and report the bad.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SubagentScan {
    pub definitions: Vec<SubagentDefinition>,
    pub unreadable:  Vec<Unreadable>,
}

impl SubagentScan {
    /// Whether the scan found nothing at all, readable or otherwise.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty() && self.unreadable.is_empty()
    }
}

/// Reads one coding-agent tool's on-disk subagent definitions.
///
/// Implementations are strictly read-only: they create, modify and delete
/// nothing in the tool they read from. A missing directory is not an error -
/// the tool may simply not be installed - and yields an empty scan.
pub trait SubagentProvider {
    /// Every definition this tool holds, from the user-level location and,
    /// when `folder` is given, that folder's project-level one.
    fn definitions(&self, folder: Option<&Path>) -> SubagentScan;

    /// Whether this tool has anything at all to offer for `folder`.
    ///
    /// Defined in terms of [`Self::definitions`] so the two can never
    /// disagree about whether there is something to show.
    fn has_definitions(&self, folder: Option<&Path>) -> bool {
        !self.definitions(folder).is_empty()
    }
}

/// Resolve a subagent-definition provider by tool. `None` for a tool with no
/// reader.
///
/// UNWIRED(#287): `Codex`, `OpenCode` and `Gemini` are part of this change but
/// deliberately unresolved. Their formats could not be read from a real
/// installation - nothing on the development machine has subagent definitions
/// for any of the three - and `openspec/changes/import-from-other-tools/
/// design.md` refuses to model a format from inference, because a reader for a
/// format that does not exist finds nothing, which looks exactly like a user
/// with no definitions. Tasks 4.1-4.3 add each one once its format has been
/// read.
#[must_use]
pub fn provider(tool: SubagentTool) -> Option<Box<dyn SubagentProvider>> {
    match tool {
        SubagentTool::Claude => Some(Box::new(ClaudeSubagentProvider)),
        SubagentTool::Codex | SubagentTool::OpenCode | SubagentTool::Gemini => None,
    }
}

/// Whether subagent import is available for `tool`.
#[must_use]
pub fn supports_subagent_import(tool: SubagentTool) -> bool {
    provider(tool).is_some()
}
