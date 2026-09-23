//! The vocabulary the panel prompt's slash lookup completes over: knot's
//! built-in commands and the skills the selected agent can reach.
//!
//! Contract: `openspec/specs/panel-slash-commands/spec.md` (proposed in
//! `openspec/changes/panel-slash-command-lookup`).
//!
//! - [`builtin`] supplies knot's own command tokens.
//! - [`skills`] reads `SKILL.md` frontmatter out of an agent's skill roots.
//! - [`token`] finds the slash token under the caret in the prompt buffer.
//! - [`shell`] recognises the `!` that makes a buffer a shell command
//!   (`openspec/specs/panel-shell-passthrough/spec.md`).
//!
//! Everything here deals in text, so nothing in this module can execute a
//! command or reach an agent's internals. The popup completes and the agent
//! interprets; `shell` says what a buffer *is*, and the panel decides what to
//! do about it.

mod builtin;
mod shell;
mod skills;
mod token;

use std::path::Path;

pub(crate) use shell::{can_send, has_shell_trigger, is_shell_command, shell_command};
pub(crate) use token::ActiveToken;
pub(crate) use token::active_token;

/// One completable entry: what gets inserted, and what it is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LookupEntry {
    /// The token, without its leading slash - the lookup adds that back on
    /// insertion, so a source never has to spell it.
    pub(crate) token:       String,
    /// One line saying what the entry does, shown beside the token.
    pub(crate) description: String,
}

impl LookupEntry {
    pub(crate) fn new(token: impl Into<String>, description: impl Into<String>) -> Self {
        Self { token:       token.into(),
               description: description.into(), }
    }

    /// Whether `filter` (the text typed after the `/`, without the slash)
    /// selects this entry. Case-insensitive substring over token *and*
    /// description, per the spec's "token or description matches" - no
    /// fuzzy ranking, deliberately.
    fn matches(&self, filter: &str) -> bool {
        if filter.is_empty() {
            return true;
        }
        let filter = filter.to_lowercase();
        self.token.to_lowercase().contains(&filter)
        || self.description.to_lowercase().contains(&filter)
    }
}

/// A place entries come from. A third source later is a new implementor,
/// not a change to the popup - which is the whole point of the trait.
pub(crate) trait LookupSource {
    fn entries(&self) -> Vec<LookupEntry>;
}

/// The entries one lookup draws on, collected once per focused agent.
///
/// Collection reads the filesystem (see [`skills`]), so a registry is built
/// off the render path and cached - never rebuilt per keystroke.
#[derive(Clone, Debug, Default)]
pub(crate) struct LookupRegistry {
    entries: Vec<LookupEntry>,
}

impl LookupRegistry {
    /// Collects every source in order, dropping an entry whose token an
    /// earlier source already claimed - so a skill cannot shadow a built-in
    /// command.
    pub(crate) fn from_sources(sources: &[&dyn LookupSource]) -> Self {
        let mut entries: Vec<LookupEntry> = Vec::new();
        for source in sources {
            for entry in source.entries() {
                if entries.iter().any(|seen| seen.token == entry.token) {
                    continue;
                }
                entries.push(entry);
            }
        }
        Self { entries }
    }

    /// The registry for the agent whose folder is `folder`: knot's built-in
    /// commands plus whatever skills that agent's roots hold.
    pub(crate) fn for_agent(folder: Option<&Path>) -> Self {
        let builtin = builtin::BuiltinCommands;
        let skills = skills::SkillRoots::for_agent(folder);
        Self::from_sources(&[&builtin, &skills])
    }

    /// The entries `filter` selects, in registry order.
    pub(crate) fn matching(&self, filter: &str) -> Vec<&LookupEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.matches(filter))
            .collect()
    }
}

#[cfg(test)]
mod tests;
