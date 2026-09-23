//! The vocabularies the panel prompt's lookup completes over: knot's
//! built-in commands, the skills the selected agent can reach, and the
//! files in its working folder.
//!
//! Contract: `openspec/specs/panel-slash-commands/spec.md` and
//! `openspec/specs/panel-file-mentions/spec.md`.
//!
//! - [`builtin`] supplies knot's own command tokens.
//! - [`skills`] reads `SKILL.md` frontmatter out of an agent's skill roots.
//! - [`entry`] is what an entry is, and how a filter narrows a source.
//! - [`files`] enumerates the agent's working folder for `@` mentions.
//! - [`fuzzy`] is the subsequence matching and score file paths need.
//! - [`registry`] joins sources into the list one lookup draws on.
//! - [`token`] finds the token under the caret and which trigger it is.
//!
//! Everything here deals in text - a token and a description - so nothing
//! in this module can execute a command or reach an agent's internals. The
//! popup completes; the agent interprets.

mod builtin;
mod entry;
mod files;
mod fuzzy;
mod registry;
mod skills;
mod token;

#[cfg(test)]
mod tests;

pub(crate) use entry::LookupEntry;
pub(crate) use entry::LookupMatch;
pub(crate) use entry::LookupSource;
pub(crate) use files::FolderFiles;
pub(crate) use registry::LookupRegistry;
pub(crate) use token::ActiveToken;
pub(crate) use token::Trigger;
pub(crate) use token::active_token;
