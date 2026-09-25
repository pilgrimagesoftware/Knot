//! The bench from a workspace window: saving an agent to it, benching one
//! (save, then close), and deploying an entry back into this workspace.
//!
//! Contract: `openspec/specs/agent-lifecycle/spec.md` ("Benching an agent",
//! "Bench deployment") and `openspec/specs/agent-list-ui/spec.md`.
//!
//! Every write goes through `settings_global::write_persisting`, onto the
//! live settings surface: the bench is also edited from the Settings window
//! and the bench popover, and a snapshot of it would drop their changes.

use std::path::Path;

use gpui_kit::App;
use knot_agents::Agent;
use knot_core::BenchAgent;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// The bench entry Save to Bench and Bench Agent both write for `agent`:
/// one function, so the two items cannot record different fields.
///
/// Registry metadata and the startup prompt travel with the template, so a
/// saved entry records a role and not just a folder. See
/// `openspec/specs/agent-registry/spec.md`.
pub(crate) fn bench_entry_for(agent: &Agent) -> BenchAgent {
    let mut entry = BenchAgent::new(Uuid::new_v4(),
                                    agent.name.clone(),
                                    Some(agent.avatar.clone()),
                                    agent.folder.clone());
    entry.agent_type = agent.agent_type.clone();
    entry.shell_command = agent.shell_command.clone();
    entry.persona_id = agent.persona_id;
    entry.description = agent.description.clone();
    entry.capabilities = agent.capabilities.clone();
    entry.cost_tier = agent.cost_tier;
    entry.startup_prompt = agent.startup_prompt.clone();
    entry
}

impl WorkspaceWindow {
    /// Save agent `id` to the bench, replacing any entry for its folder.
    pub(in crate::workspace_window) fn save_to_bench(&self, id: Uuid, cx: &App)
                                                     -> knot_core::Result<()> {
        let Some(entry) = self.store.lock().agent(id).map(bench_entry_for)
        else {
            return Ok(());
        };
        crate::settings_global::write_persisting(cx, |settings| settings.add_bench_agent(entry))
    }

    /// Bench agent `id`: save it to the bench, then close it and its
    /// companions. The entry is written first, and if that fails the agent
    /// is left running - closing it after a failed write would lose its
    /// configuration entirely.
    pub(in crate::workspace_window) fn bench_agent(&mut self, id: Uuid, cx: &App)
                                                   -> knot_core::Result<()> {
        self.save_to_bench(id, cx)?;
        self.remove_agent(id, cx);
        Ok(())
    }

    /// Deploy `entry` into this window's workspace and select the new agent.
    ///
    /// When the entry's folder no longer exists, the entry is removed from
    /// the bench and the sidebar's error line says which one and why, rather
    /// than the click doing nothing visible.
    pub(in crate::workspace_window) fn deploy_bench_entry(&mut self, entry: &BenchAgent, cx: &App) {
        let created = self.store
                          .lock()
                          .deploy_bench(entry, Some(self.workspace_id), |folder| {
                              is_directory(folder)
                          });
        match created {
            Some(id) => {
                self.persist_agents(cx);
                self.select_agent(id);
            }
            None => {
                if let Err(error) = crate::settings_global::write_persisting(cx, |settings| {
                    settings.remove_bench_agent(entry.id)
                }) {
                    eprintln!("failed to prune a stale bench entry: {error}");
                }
                self.error = Some(knot_core::l10n::t_with("sidebar.bench_entry_missing",
                                                          &[("name", &entry.name),
                                                            ("folder", &entry.folder)]));
            }
        }
    }
}

/// Blocking, but a single `stat` on a click, not on the render path.
fn is_directory(folder: &Path) -> bool {
    folder.is_dir()
}

#[cfg(test)]
mod tests;
