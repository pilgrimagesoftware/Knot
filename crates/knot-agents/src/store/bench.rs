use std::path::Path;

use knot_core::BenchAgent;
use uuid::Uuid;

use super::{AgentStore, CreateOptions};

impl AgentStore {
    /// Create an agent from `bench`, in `workspace_id` when given, or `None`
    /// when its folder no longer exists - the caller then prunes the entry,
    /// per `agent-lifecycle`'s "Bench deployment". The entry itself is not
    /// consumed: one entry deploys any number of times.
    pub fn deploy_bench(&mut self, bench: &BenchAgent, workspace_id: Option<Uuid>,
                        folder_exists: impl FnOnce(&Path) -> bool)
                        -> Option<Uuid> {
        if !folder_exists(Path::new(&bench.folder)) {
            return None;
        }
        Some(self.create(bench.folder.clone(),
                         CreateOptions { name: Some(bench.name.clone()),
                                         avatar: Some(bench.avatar.clone()),
                                         agent_type: Some(bench.agent_type.clone()),
                                         shell_command: bench.shell_command.clone(),
                                         persona_id: bench.persona_id,
                                         description: bench.description.clone(),
                                         capabilities: bench.capabilities.clone(),
                                         cost_tier: bench.cost_tier,
                                         startup_prompt: bench.startup_prompt.clone(),
                                         workspace_id,
                                         ..Default::default() }))
    }
}
