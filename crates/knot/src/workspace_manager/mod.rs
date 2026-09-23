//! The Workspace Manager window - the roster of workspaces, and the two
//! dialogs that change it (`openspec/specs/workspace-manager-ui`).
//!
//! [`state`] is what the window knows, [`actions`] is what its rows and
//! toolbar do, [`dialog`] is the name dialog for creating and renaming, and
//! [`render`] is what all of it draws.

mod actions;
mod dialog;
mod render;
mod state;

pub(crate) use state::WorkspaceDrag;
pub(crate) use state::WorkspaceDragPreview;
pub(crate) use state::WorkspaceManager;
pub(crate) use state::workspace_name_is_blank;
