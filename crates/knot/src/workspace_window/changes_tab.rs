//! The Changes view's tabs (#504): which list the view is showing, held per
//! workspace window and never persisted, so a relaunched window opens on
//! Pull Requests.
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/pull-request-tracking/spec.md`,
//! "The Changes view is divided into tabs".

use std::fmt;
use std::str::FromStr;

/// One tab of the Changes view, in the order the tab bar shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ChangesTab {
    #[default]
    PullRequests,
    Issues,
    OpenSpec,
}

impl ChangesTab {
    pub(crate) const ALL: [Self; 3] = [Self::PullRequests, Self::Issues, Self::OpenSpec];

    /// A stable identifier, for `FromStr` and element ids. Not the label.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::PullRequests => "pull-requests",
            Self::Issues => "issues",
            Self::OpenSpec => "openspec",
        }
    }

    /// The tab bar's label.
    pub(crate) fn label(self) -> String {
        knot_core::l10n::t(match self {
                               Self::PullRequests => "changes_view.tab.pull_requests",
                               Self::Issues => "changes_view.tab.issues",
                               Self::OpenSpec => "changes_view.tab.openspec",
                           })
    }

    /// Where this tab sits in the bar.
    pub(crate) fn index(self) -> usize {
        Self::ALL.iter()
                 .position(|tab| *tab == self)
                 .expect("every tab is in ALL")
    }
}

impl fmt::Display for ChangesTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for ChangesTab {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter()
                 .find(|tab| tab.id() == value)
                 .ok_or_else(|| format!("no Changes tab named {value:?}"))
    }
}

impl super::WorkspaceWindow {
    /// Whether the window is showing the Changes view on `tab` - the gate
    /// every tab's fetching sits behind, so nothing is fetched for a tab the
    /// user cannot see.
    pub(super) fn showing_changes_tab(&self, tab: ChangesTab) -> bool {
        self.view_mode == super::WorkspaceViewMode::Changes && self.changes_view.tab == tab
    }
}

/// The Changes view's per-window state beyond the Pull Requests tab's own
/// (`PullRequestViewState`, which predates the tabs and stays where it is).
#[derive(Default)]
pub(crate) struct ChangesViewState {
    /// The tab shown. Survives leaving the view and returning; a new window
    /// starts on [`ChangesTab::PullRequests`].
    pub(crate) tab: ChangesTab,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_window_starts_on_pull_requests() {
        assert_eq!(ChangesViewState::default().tab, ChangesTab::PullRequests);
    }

    #[test]
    fn the_tabs_are_in_bar_order() {
        assert_eq!(ChangesTab::ALL.map(ChangesTab::index), [0, 1, 2]);
        assert_eq!(ChangesTab::ALL,
                   [ChangesTab::PullRequests, ChangesTab::Issues, ChangesTab::OpenSpec]);
    }

    #[test]
    fn tabs_round_trip_through_their_ids() {
        for tab in ChangesTab::ALL {
            assert_eq!(tab.to_string().parse(), Ok(tab));
        }
        assert!("Pull Requests".parse::<ChangesTab>().is_err());
    }

    #[test]
    fn every_tab_label_resolves() {
        for tab in ChangesTab::ALL {
            assert!(!tab.label().starts_with("changes_view."), "{tab} rendered its key");
        }
    }
}
