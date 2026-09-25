//! The bench-templates collection: its document and the helpers that edit it.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`, "Bench templates".
//! Deploying an entry is the agent store's business, not the settings
//! store's; see `knot-agents`.

use uuid::Uuid;

use super::{BenchAgent, Settings, documents};
use crate::error::Result;
use crate::settings::prompts::StartupPrompt;

impl Settings {
    pub(crate) fn persist_bench(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.bench(), &self.bench_agents)
    }

    /// Add a bench template, replacing any existing entry for the same folder.
    pub fn add_bench_agent(&mut self, entry: BenchAgent) -> Result<()> {
        self.bench_agents.retain(|b| b.folder != entry.folder);
        self.bench_agents.push(entry);
        self.persist_bench()
    }

    /// Remove the bench entry with `id`. Removing an id that is not on the
    /// bench changes nothing and writes nothing.
    pub fn remove_bench_agent(&mut self, id: Uuid) -> Result<()> {
        let before = self.bench_agents.len();
        self.bench_agents.retain(|b| b.id != id);
        if self.bench_agents.len() == before {
            return Ok(());
        }
        self.persist_bench()
    }

    /// Change a bench entry's name and startup prompt - the two fields the
    /// Bench tab edits; everything else comes from the agent it was saved
    /// from. An unknown id changes nothing.
    pub fn update_bench_agent(&mut self, id: Uuid, name: impl Into<String>,
                              startup_prompt: Option<StartupPrompt>)
                              -> Result<()> {
        let Some(entry) = self.bench_agents.iter_mut().find(|b| b.id == id)
        else {
            return Ok(());
        };
        entry.name = name.into();
        entry.startup_prompt = startup_prompt;
        self.persist_bench()
    }
}
