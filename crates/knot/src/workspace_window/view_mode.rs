//! Which peer view a workspace window is showing.

/// Which peer view a `WorkspaceWindow` currently shows - each is a
/// toggleable view of the same window's content, not a dialog or a separate
/// window (see `openspec/changes/dashboard-view/design.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkspaceViewMode {
    #[default]
    Terminal,
    Dashboard,
    /// The pull requests this workspace's agents have opened. Reached from
    /// its own sidebar row, shaped like the dashboard's.
    PullRequests,
}

impl WorkspaceViewMode {
    /// Whether this mode replaces the window's content, hiding the selected
    /// agent's header and pane.
    ///
    /// The question every render actually asks, rather than `== Dashboard`
    /// spelled out in five places - which is how a third mode gets missed in
    /// four of them.
    pub(crate) fn is_takeover(self) -> bool {
        !matches!(self, Self::Terminal)
    }

    /// Toggle into `target`, or back to the terminal if it is already
    /// showing. What every launcher row in the sidebar does.
    pub(crate) fn toggled(self, target: Self) -> Self {
        if self == target {
            Self::Terminal
        }
        else {
            target
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkspaceViewMode;

    #[test]
    fn only_the_terminal_is_not_a_takeover() {
        assert!(!WorkspaceViewMode::Terminal.is_takeover());
        assert!(WorkspaceViewMode::Dashboard.is_takeover());
        assert!(WorkspaceViewMode::PullRequests.is_takeover());
    }

    #[test]
    fn a_row_toggles_its_own_view_and_switches_between_views() {
        let terminal = WorkspaceViewMode::Terminal;
        let dashboard = WorkspaceViewMode::Dashboard;
        let pull_requests = WorkspaceViewMode::PullRequests;

        assert_eq!(terminal.toggled(dashboard), dashboard);
        assert_eq!(dashboard.toggled(dashboard),
                   terminal,
                   "clicking the showing view's row goes back");
        assert_eq!(dashboard.toggled(pull_requests), pull_requests);
        assert_eq!(pull_requests.toggled(dashboard), dashboard);
    }
}
