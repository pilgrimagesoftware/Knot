//! The prompt-library collection: its document and the helpers that edit it.
//!
//! Contract: `openspec/specs/prompt-library/spec.md` and
//! `openspec/specs/settings-persistence/spec.md`.
//!
//! Removing a prompt deliberately leaves every agent and bench entry that
//! references it alone: rewriting them would make a library edit write three
//! documents, and would hide from the agent editor why an agent stopped
//! getting its startup prompt. A dangling reference resolves to nothing.

use uuid::Uuid;

use super::{Settings, documents};
use crate::error::{Error, Result};
use crate::settings::prompts::{Prompt, StartupPrompt};

/// How many saved agents and bench entries name one library prompt as their
/// startup prompt - what the Prompts tab's delete confirmation reports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PromptReferences {
    pub agents: usize,
    pub bench:  usize,
}

impl PromptReferences {
    pub fn is_empty(&self) -> bool {
        self.agents == 0 && self.bench == 0
    }
}

impl Settings {
    /// Write the prompt-library document.
    ///
    /// Skipped while the library is empty and no document exists, so an
    /// installation that never used the library grows no file for it. Once
    /// the document exists it is always written, so removing the last prompt
    /// survives a restart.
    pub(crate) fn persist_prompts(&self) -> Result<()> {
        let path = self.resolved_paths()?.prompts();
        if self.prompts.is_empty() && !path.exists() {
            return Ok(());
        }
        documents::write_collection(&path, &self.prompts)
    }

    /// Append a prompt to the library and persist it, returning its id.
    /// Refused, leaving the library unchanged, when the name or text is blank.
    pub fn add_prompt(&mut self, name: impl Into<String>, text: impl Into<String>) -> Result<Uuid> {
        let prompt = Prompt::new(name, text).ok_or_else(blank_prompt)?;
        let id = prompt.id;
        self.prompts.push(prompt);
        self.persist_prompts()?;
        Ok(id)
    }

    /// Rewrite a prompt's name and text, keeping its id and position. Refused
    /// when either is blank; an unknown id changes nothing.
    pub fn update_prompt(&mut self, id: Uuid, name: impl Into<String>, text: impl Into<String>)
                         -> Result<()> {
        let replacement = Prompt::new(name, text).ok_or_else(blank_prompt)?;
        let Some(prompt) = self.prompts.iter_mut().find(|p| p.id == id)
        else {
            return Ok(());
        };
        prompt.name = replacement.name;
        prompt.text = replacement.text;
        self.persist_prompts()
    }

    /// Remove a prompt from the library. References to it are kept; see the
    /// module doc. An unknown id changes nothing and writes nothing.
    pub fn remove_prompt(&mut self, id: Uuid) -> Result<()> {
        let before = self.prompts.len();
        self.prompts.retain(|p| p.id != id);
        if self.prompts.len() == before {
            return Ok(());
        }
        self.persist_prompts()
    }

    /// The library prompt with `id`, if it is still in the library.
    pub fn prompt(&self, id: Uuid) -> Option<&Prompt> {
        self.prompts.iter().find(|p| p.id == id)
    }

    /// How many saved agents and bench entries reference prompt `id`.
    pub fn prompt_references(&self, id: Uuid) -> PromptReferences {
        let names = |startup: &Option<StartupPrompt>| matches!(startup, Some(StartupPrompt::Library(target)) if *target == id);
        PromptReferences { agents: self.saved_agents
                                       .iter()
                                       .filter(|a| names(&a.startup_prompt))
                                       .count(),
                           bench:  self.bench_agents
                                       .iter()
                                       .filter(|b| names(&b.startup_prompt))
                                       .count(), }
    }
}

fn blank_prompt() -> Error {
    Error::Config("a prompt needs a name and text".to_string())
}
