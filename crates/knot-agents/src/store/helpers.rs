use std::path::Path;

use knot_core::Workspace;
use uuid::Uuid;

/// The workspace a store with agents but no saved workspace falls back to.
///
/// Configuration only: a fresh workspace has no arrangement, which is what
/// [`knot_core::WorkspaceUiState::default`] already means. The one piece the
/// caller may want - the first agent showing - is set through
/// `set_workspace_layout` rather than fabricated here, so there is one place
/// that writes UI state.
pub(super) fn default_workspace(agent_ids: Vec<Uuid>) -> Workspace {
    Workspace { id: Uuid::new_v4(),
                name: super::DEFAULT_WORKSPACE_NAME.to_string(),
                color_hex: super::DEFAULT_WORKSPACE_COLOR.to_string(),
                agent_ids }
}

pub(super) fn last_path_component(folder: &str) -> String {
    Path::new(folder).file_name()
                     .map(|name| name.to_string_lossy().into_owned())
                     .unwrap_or_else(|| folder.to_string())
}
