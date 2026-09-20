use std::path::Path;

use knot_core::Workspace;
use uuid::Uuid;

pub(super) fn default_workspace(agent_ids: Vec<Uuid>) -> Workspace {
    Workspace { id: Uuid::new_v4(),
                name: super::DEFAULT_WORKSPACE_NAME.to_string(),
                color_hex: super::DEFAULT_WORKSPACE_COLOR.to_string(),
                active_agent_ids: agent_ids.first().copied().into_iter().collect(),
                agent_ids,
                layout_mode: "single".to_string(),
                focused_pane_index: 0,
                split_ratio: 0.5,
                split_ratio_secondary: None,
                show_dashboard: None,
                is_detached: None,
                window_bounds: None }
}

pub(super) fn last_path_component(folder: &str) -> String {
    Path::new(folder).file_name()
                     .map(|name| name.to_string_lossy().into_owned())
                     .unwrap_or_else(|| folder.to_string())
}
