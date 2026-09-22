//! Which peer view a workspace window is showing.

/// Which peer view a `WorkspaceWindow` currently shows - the dashboard is a
/// toggleable view of the same window's content, not a dialog or a
/// separate window (see `openspec/changes/dashboard-view/design.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkspaceViewMode {
    #[default]
    Terminal,
    Dashboard,
}
