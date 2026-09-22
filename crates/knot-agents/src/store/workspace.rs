use uuid::Uuid;

use super::AgentStore;

impl AgentStore {
    /// The workspace `id` names, for the operations that edit one in place.
    /// The sibling of `agent_mut`, which the agent half of the store has
    /// had all along - four lookups here were written out by hand instead.
    fn workspace_mut(&mut self, id: Uuid) -> Option<&mut knot_core::Workspace> {
        self.workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
    }

    pub fn add_workspace(&mut self, workspace: knot_core::Workspace) {
        self.workspaces.push(workspace);
    }

    pub fn current_workspace_id(&self) -> Option<Uuid> {
        self.current_workspace_id
    }

    pub fn set_current_workspace(&mut self, id: Uuid) {
        self.current_workspace_id = Some(id);
    }

    /// Records where `id`'s window was last seen, so reopening the
    /// workspace restores its frame. Returns whether the workspace exists
    /// and the frame actually changed - the caller persists only then,
    /// since bounds observers fire continuously through a drag.
    pub fn set_workspace_window_bounds(&mut self, id: Uuid, bounds: knot_core::SavedWindowBounds)
                                       -> bool {
        let Some(workspace) = self.workspace_mut(id)
        else {
            return false;
        };
        if workspace.window_bounds == Some(bounds) {
            return false;
        }
        workspace.window_bounds = Some(bounds);
        true
    }

    pub fn rename_workspace(&mut self, id: Uuid, name: impl Into<String>) -> bool {
        let Some(workspace) = self.workspace_mut(id)
        else {
            return false;
        };
        workspace.name = name.into();
        true
    }

    pub fn remove_workspace(&mut self, id: Uuid) -> bool {
        if self.workspaces.len() <= 1 {
            return false;
        }
        let Some(index) = self.workspaces
                              .iter()
                              .position(|workspace| workspace.id == id)
        else {
            return false;
        };
        let workspace = self.workspaces.remove(index);
        for agent_id in workspace.agent_ids {
            self.remove(agent_id);
        }
        if self.current_workspace_id == Some(id) {
            self.current_workspace_id = self.workspaces
                                            .get(index.saturating_sub(1))
                                            .or_else(|| self.workspaces.first())
                                            .map(|workspace| workspace.id);
        }
        true
    }

    pub fn move_workspace_before(&mut self, id: Uuid, target_id: Uuid) -> bool {
        if id == target_id {
            return false;
        }
        let Some(source_index) = self.workspaces
                                     .iter()
                                     .position(|workspace| workspace.id == id)
        else {
            return false;
        };
        let Some(target_index) = self.workspaces
                                     .iter()
                                     .position(|workspace| workspace.id == target_id)
        else {
            return false;
        };
        let workspace = self.workspaces.remove(source_index);
        let insertion_index = if source_index < target_index {
            target_index - 1
        }
        else {
            target_index
        };
        self.workspaces.insert(insertion_index, workspace);
        true
    }

    pub(super) fn ensure_current_workspace(&mut self) -> Uuid {
        if let Some(id) = self.current_workspace_id
           && self.workspaces.iter().any(|workspace| workspace.id == id)
        {
            return id;
        }
        let workspace = super::helpers::default_workspace(Vec::new());
        let id = workspace.id;
        self.workspaces.push(workspace);
        self.current_workspace_id = Some(id);
        id
    }

    pub(super) fn workspace_of(&self, agent_id: Uuid) -> Option<Uuid> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.agent_ids.contains(&agent_id))
            .map(|workspace| workspace.id)
    }

    pub fn move_to_workspace(&mut self, agent_id: Uuid, target_workspace_id: Uuid) {
        if let Some(source) = self.workspace_of(agent_id) {
            if source == target_workspace_id {
                return;
            }
            if let Some(workspace) = self.workspace_mut(source) {
                workspace.agent_ids.retain(|id| *id != agent_id);
                workspace.active_agent_ids.retain(|id| *id != agent_id);
            }
        }
        if let Some(workspace) = self.workspace_mut(target_workspace_id) {
            workspace.agent_ids.push(agent_id);
            if workspace.active_agent_ids.is_empty() {
                workspace.active_agent_ids = vec![agent_id];
            }
        }
    }
}
