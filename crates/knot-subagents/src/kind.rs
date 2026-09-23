//! What kind of subagent was dispatched.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "A subagent is a typed
//! record, not free text".
//!
//! An *open* vocabulary, unlike [`crate::SubagentState`]: the set of subagent
//! kinds is the user's own configuration - the files under `~/.claude/agents`
//! and whatever each tool adds next - so there is no enum to close it with.
//! `agent_type` is the same case and is handled the same way in
//! `knot_core::agent_type`: keep the string, and give the one thing the code
//! genuinely decides a type of its own.
//!
//! The thing the code decides here is whether a kind was stated at all. That
//! is why this is not simply a `String`: a report naming no kind and a report
//! naming the kind `"unstated"` are different facts, and an empty string
//! makes them the same one.

use std::fmt;

#[cfg(test)]
mod tests;

/// The kind of subagent a report named, or the absence of one.
///
/// [`Unstated`] is a variant rather than an empty [`Named`] so that the
/// absence cannot be confused with a kind whose name happens to be empty, and
/// so that no call site has to remember which sentinel string means "none".
///
/// [`Unstated`]: Self::Unstated
/// [`Named`]: Self::Named
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubagentKind {
    /// The report named no kind. The row says so in the user's language; the
    /// word for it is the catalogue's, never this crate's.
    Unstated,
    Named(String),
}

impl SubagentKind {
    /// Builds a kind from what a report carried.
    ///
    /// A missing field, an empty string and a string of only whitespace all
    /// mean the same thing - the agent named no kind - and all become
    /// [`Unstated`]. Trimming here rather than at each call site is what keeps
    /// `" "` from reaching the header's name list as an invisible entry.
    ///
    /// [`Unstated`]: Self::Unstated
    pub fn from_reported(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            None | Some("") => Self::Unstated,
            Some(name) => Self::Named(name.to_owned()),
        }
    }

    /// The name the agent gave, or `None` when it gave none.
    ///
    /// Returning an `Option` rather than a placeholder string is what forces
    /// the caller to reach for the localization catalogue for the absent case.
    /// A subagent's kind is the agent's own data and is shown verbatim; the
    /// word for *having no kind* is ours, and ours must be translated.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Unstated => None,
            Self::Named(name) => Some(name),
        }
    }

    pub const fn is_stated(&self) -> bool {
        matches!(self, Self::Named(_))
    }
}

impl fmt::Display for SubagentKind {
    /// Writes the name, and writes nothing at all when there is none.
    ///
    /// Deliberately not "unknown" or "-": this type has no business choosing
    /// English, and a `Display` that emitted one would be the easiest way for
    /// an untranslated word to reach the screen.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name().unwrap_or(""))
    }
}
