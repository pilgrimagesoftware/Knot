use std::path::Path;

use knot_core::BenchAgent;
use uuid::Uuid;

use super::{AgentStore, CreateOptions};

impl AgentStore {
    pub fn deploy_bench(&mut self, bench: &BenchAgent, folder_exists: impl FnOnce(&Path) -> bool)
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
                                         ..Default::default() }))
    }
}
