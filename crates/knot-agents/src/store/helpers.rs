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

/// The name an agent created in `folder` takes when its caller supplied
/// none.
///
/// The rule itself is `knot_core::folder_name`, shared with the agent editor
/// so the dialog and the store cannot name the same folder differently. What
/// stays here is only this caller's answer to a folder with no last
/// component - the path itself, which is what the store has always used and
/// what a caller with no name field to leave blank needs.
pub(super) fn name_for_folder(folder: &str) -> String {
    knot_core::folder_name(folder).unwrap_or_else(|| folder.to_string())
}
