//! The startup prompt: resolving an agent's [`StartupPrompt`] against the
//! library, building the [`PromptContext`] its variables expand with, and
//! the one-line form the terminal path types in.
//!
//! Contract: `openspec/specs/prompt-library/spec.md` and
//! `openspec/specs/agent-launch-command/spec.md`, "Startup prompt follows the
//! initialization prompt".
//!
//! [`read_context`] runs `git` and must be called off the UI thread; the rest
//! is pure.

use std::path::Path;

use knot_core::{Prompt, StartupPrompt};
use time::OffsetDateTime;
use time::macros::format_description;

use crate::variables::PromptContext;

/// The raw text an agent's startup prompt stands for, variables unexpanded:
/// nothing for none, the referenced prompt's current text for a reference,
/// and nothing for a reference to a prompt no longer in the library.
pub fn resolve_startup_prompt(startup: Option<&StartupPrompt>, library: &[Prompt])
                              -> Option<String> {
    match startup? {
        StartupPrompt::Library(id) => library.iter()
                                             .find(|prompt| prompt.id == *id)
                                             .map(|prompt| prompt.text.clone()),
        StartupPrompt::Custom(text) => Some(text.clone()),
    }
}

/// `text` on one line, each line break (and the whitespace around it)
/// replaced by a single space.
///
/// For the terminal path, which types the prompt and then sends Return: an
/// embedded newline would submit it half-written, the same reason
/// `knot_instructions` stays on one line. Applied after expansion, so a
/// multi-line value is flattened too.
pub fn one_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// What an agent contributes to its own [`PromptContext`]; the branch and
/// the date are read by [`read_context`].
#[derive(Debug, Clone, Default)]
pub struct ContextSource {
    pub agent_name: String,
    pub agent_id:   String,
    pub agent_type: String,
    pub folder:     String,
    pub workspace:  Option<String>,
}

/// The context for `source`, reading its folder's branch and today's date.
///
/// Blocking: runs `git`. Call from a runtime or blocking thread.
pub fn read_context(source: ContextSource) -> PromptContext {
    let branch = read_branch(Path::new(&source.folder));
    PromptContext { agent_name: source.agent_name,
                    agent_id: source.agent_id,
                    agent_type: source.agent_type,
                    folder: source.folder,
                    workspace: source.workspace.unwrap_or_default(),
                    branch,
                    date: today() }
}

/// The folder's current branch; the short commit id when HEAD is detached;
/// empty when it is not in a git repository or `git` cannot answer.
fn read_branch(folder: &Path) -> String {
    let repo = knot_git::Repository::open(folder);
    match repo.current_branch() {
        Ok(Some(branch)) => branch,
        Ok(None) => repo.short_head().unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// Today as `YYYY-MM-DD` in the local time zone, or in UTC when the local
/// offset cannot be determined - `time` refuses it where reading it could
/// race another thread's environment writes, and a date a few hours off
/// beats no date.
fn today() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    now.format(format_description!("[year]-[month]-[day]"))
       .unwrap_or_default()
}

#[cfg(test)]
mod tests;
