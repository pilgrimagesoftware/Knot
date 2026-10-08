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

    /// Add a bench template, replacing an existing entry only when it is the
    /// same agent saved again - same name and same folder.
    ///
    /// Not the folder alone, which is where the Swift reference stopped:
    /// Knot's agents commonly share a repository folder, and keying on it
    /// made saving a second agent from that repository overwrite the first
    /// (#500). A name is unique among the agents sharing a folder, so name and
    /// folder together tell a re-save from a different agent.
    pub fn add_bench_agent(&mut self, entry: BenchAgent) -> Result<()> {
        self.bench_agents
            .retain(|b| !(b.folder == entry.folder && b.name == entry.name));
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
