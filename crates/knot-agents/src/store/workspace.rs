use knot_core::WorkspaceUiState;
use uuid::Uuid;

use super::AgentStore;

impl AgentStore {
    /// The UI state for `id`, created at its defaults if the workspace has
    /// never been arranged.
    ///
    /// Not gated on the workspace existing: the setters below are driven by
    /// window observers, which can fire for a workspace closed a moment ago,
    /// and an entry with no workspace is pruned on the next load rather than
    /// guarded against on every write.
    fn workspace_ui_mut(&mut self, id: Uuid) -> &mut WorkspaceUiState {
        self.workspace_ui.entry(id).or_default()
    }

    /// How `id`'s window is arranged, or the defaults if it never has been.
    pub fn workspace_ui(&self, id: Uuid) -> WorkspaceUiState {
        self.workspace_ui.get(&id).cloned().unwrap_or_default()
    }

    /// Replace the whole UI-state map, as the store is loaded from settings.
    pub fn set_workspace_ui(&mut self, ui: std::collections::BTreeMap<Uuid, WorkspaceUiState>) {
        self.workspace_ui = ui;
    }

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
    /// workspace restores its frame. Returns whether the frame actually
    /// changed - the caller persists only then, since bounds observers fire
    /// continuously through a drag.
    pub fn set_workspace_window_bounds(&mut self, id: Uuid, bounds: knot_core::SavedWindowBounds)
                                       -> bool {
        let ui = self.workspace_ui_mut(id);
        if ui.window_bounds == Some(bounds) {
            return false;
        }
        ui.window_bounds = Some(bounds);
        true
    }

    /// Record `id`'s layout mode and which agents are active in it.
    pub fn set_workspace_layout(&mut self, id: Uuid, layout_mode: impl Into<String>,
                                active_agent_ids: Vec<Uuid>) {
        let ui = self.workspace_ui_mut(id);
        ui.layout_mode = layout_mode.into();
        ui.active_agent_ids = active_agent_ids;
    }

    /// Record which agents `id`'s layout is showing, leaving its layout mode
    /// alone.
    pub fn set_workspace_active_agents(&mut self, id: Uuid, active_agent_ids: Vec<Uuid>) {
        self.workspace_ui_mut(id).active_agent_ids = active_agent_ids;
    }

    /// Record which pane of `id`'s layout has focus.
    pub fn set_workspace_focused_pane(&mut self, id: Uuid, index: i32) {
        self.workspace_ui_mut(id).focused_pane_index = index;
    }

    /// Record `id`'s split ratios - the secondary one only applies to the
    /// three- and four-pane layouts, hence the `Option`.
    pub fn set_workspace_split_ratios(&mut self, id: Uuid, primary: f64, secondary: Option<f64>) {
        let ui = self.workspace_ui_mut(id);
        ui.split_ratio = primary;
        ui.split_ratio_secondary = secondary;
    }

    /// Record whether `id` is showing its dashboard.
    pub fn set_workspace_show_dashboard(&mut self, id: Uuid, showing: bool) {
        self.workspace_ui_mut(id).show_dashboard = Some(showing);
    }

    /// Record whether `id` is detached into its own window.
    pub fn set_workspace_detached(&mut self, id: Uuid, detached: bool) {
        self.workspace_ui_mut(id).is_detached = Some(detached);
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
        // Removed here as well as pruned on load: this is the teardown that
        // already drops per-workspace state, and leaving it to load alone
        // would let a window reopened before the next launch read the dead
        // workspace's arrangement.
        self.workspace_ui.remove(&id);
        for agent_id in workspace.agent_ids {
            self.remove(agent_id);
        }
        self.forget_workspace_pull_requests(id);
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
            }
            self.workspace_ui_mut(source)
                .active_agent_ids
                .retain(|id| *id != agent_id);
        }
        let mut became_only_member = false;
        if let Some(workspace) = self.workspace_mut(target_workspace_id) {
            workspace.agent_ids.push(agent_id);
            became_only_member = true;
        }
        if became_only_member {
            let ui = self.workspace_ui_mut(target_workspace_id);
            if ui.active_agent_ids.is_empty() {
                ui.active_agent_ids = vec![agent_id];
            }
        }
    }
}
