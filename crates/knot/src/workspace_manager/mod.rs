//! The Workspace Manager window - the roster of workspaces, and the two
//! dialogs that change it (`openspec/specs/workspace-manager-ui`).
//!
//! [`state`] is what the window knows, [`actions`] is what its rows and
//! toolbar do, [`dialog`] is the name dialog for creating and renaming,
//! [`drag`] is reordering rows by dragging them, and [`render`] is what all
//! of it draws.

mod actions;
mod dialog;
mod drag;
mod render;
mod state;

// The gap arithmetic, for `tests/workspace_manager_drag.rs`.
#[cfg(test)]
pub(crate) use drag::DropEdge;
pub(crate) use drag::WorkspaceDrag;
pub(crate) use drag::WorkspaceDragPreview;
#[cfg(test)]
pub(crate) use drag::drop_gap;
#[cfg(test)]
pub(crate) use drag::drop_line_edge;
pub(crate) use state::WorkspaceManager;
pub(crate) use state::workspace_name_is_blank;
