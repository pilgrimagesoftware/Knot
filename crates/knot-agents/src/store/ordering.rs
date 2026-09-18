use uuid::Uuid;

use super::AgentStore;

impl AgentStore {
    pub fn reorder(&mut self, workspace_id: Uuid, from: usize, to: usize) {
        let Some(workspace) = self.workspaces
                                  .iter_mut()
                                  .find(|workspace| workspace.id == workspace_id)
        else {
            return;
        };
        if from >= workspace.agent_ids.len() || to >= workspace.agent_ids.len() {
            return;
        }
        let id = workspace.agent_ids.remove(from);
        workspace.agent_ids.insert(to, id);
    }
}
