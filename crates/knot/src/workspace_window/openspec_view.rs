//! The OpenSpec tab's state for a frame (#504): its search field, the groups
//! it draws, and the message that stands in for them when it has none.
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! workspace-openspec-changes/spec.md`.

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{App, AppContext, Context, Entity, Window};

use super::WorkspaceWindow;
use crate::openspec_changes::{ChangeEntry, filter};

/// One repository's heading and its changes, as drawn.
pub(super) struct ChangeGroup {
    pub(super) heading: String,
    pub(super) changes: Vec<ChangeEntry>,
}

/// What stands in for the list, when something does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OpenSpecMessage {
    /// The repositories, or their first scans, have not answered yet.
    Checking,
    /// Every repository answered, and none has an un-archived change.
    NoChanges,
    /// There are changes, and the search or filter hides them all.
    NoneMatch,
}

/// Which message shows instead of rows, or `None` to draw them.
pub(super) fn openspec_message(repos_resolved: bool, repos: usize, answered: usize,
                               total: usize, shown: usize)
                               -> Option<OpenSpecMessage> {
    if !repos_resolved || answered < repos {
        if total == 0 {
            return Some(OpenSpecMessage::Checking);
        }
    }
    else if total == 0 {
        return Some(OpenSpecMessage::NoChanges);
    }
    (total > 0 && shown == 0).then_some(OpenSpecMessage::NoneMatch)
}

/// What a frame of the tab draws.
pub(super) struct OpenSpecFrame {
    pub(super) shown:    Vec<ChangeGroup>,
    /// Every repository heading, for the picker.
    pub(super) headings: Vec<String>,
    pub(super) message:  Option<OpenSpecMessage>,
}

impl WorkspaceWindow {
    /// The search field, created on the first frame that needs it.
    pub(super) fn openspec_search(&mut self, window: &mut Window, cx: &mut Context<Self>)
                                  -> Entity<InputState> {
        if let Some(input) = &self.changes_view.openspec.search {
            return input.clone();
        }
        let input = cx.new(|cx| {
                          InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("openspec_changes.search_placeholder"))
                .clean_on_escape()
                      });
        let subscription = cx.subscribe_in(&input, window, |_: &mut Self, _, event, _, cx| {
                                 if matches!(event, InputEvent::Change) {
                                     cx.notify();
                                 }
                             });
        self.changes_view.openspec.search = Some(input.clone());
        self.changes_view.openspec._search_changes = Some(subscription);
        input
    }

    /// The tab's frame. Groups are ordered by heading, rows by name; a
    /// repository filter naming a repository no longer in the workspace goes
    /// back to all.
    pub(super) fn openspec_frame(&mut self, cx: &App) -> OpenSpecFrame {
        let repos = self.work_items.repos.get(&());
        let scanned = self.work_items.changes.snapshot();
        let search = self.changes_view
                         .openspec
                         .search
                         .as_ref()
                         .map(|input| input.read(cx).value().to_string())
                         .unwrap_or_default();
        let mut groups: Vec<ChangeGroup> = Vec::new();
        let mut answered = 0;
        for repo in repos.iter().flatten() {
            if let Some(changes) = scanned.get(&repo.common_dir) {
                answered += 1;
                if !changes.is_empty() {
                    groups.push(ChangeGroup { heading: repo.label.clone(),
                                              changes: changes.clone(), });
                }
            }
        }
        groups.sort_by(|a, b| a.heading.cmp(&b.heading));
        let headings: Vec<String> = groups.iter().map(|group| group.heading.clone()).collect();
        filter::reset_if_missing(&mut self.changes_view.openspec.repo, &headings);
        let chosen = self.changes_view.openspec.repo.clone();

        let total = groups.iter().map(|group| group.changes.len()).sum();
        let shown: Vec<ChangeGroup> =
            groups.into_iter()
                  .filter(|group| chosen.as_ref().is_none_or(|repo| *repo == group.heading))
                  .map(|group| {
                      let heading = group.heading;
                      let changes = group.changes
                                         .into_iter()
                                         .filter(|entry| filter::matches(&search, &heading, entry))
                                         .collect();
                      ChangeGroup { heading, changes }
                  })
                  .filter(|group| !group.changes.is_empty())
                  .collect();
        let shown_count = shown.iter().map(|group| group.changes.len()).sum();
        OpenSpecFrame { message: openspec_message(repos.is_some(),
                                                  repos.as_ref().map_or(0, Vec::len),
                                                  answered,
                                                  total,
                                                  shown_count),
                        shown,
                        headings }
    }

    /// Empty the search and return to all repositories.
    pub(super) fn clear_openspec_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.changes_view.openspec.repo = None;
        if let Some(search) = &self.changes_view.openspec.search {
            search.update(cx, |input, cx| input.set_value("", window, cx));
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{OpenSpecMessage, openspec_message};

    #[test]
    fn rows_draw_with_no_message() {
        assert_eq!(openspec_message(true, 2, 2, 3, 3), None);
    }

    #[test]
    fn nothing_answered_yet_is_checking() {
        assert_eq!(openspec_message(false, 0, 0, 0, 0),
                   Some(OpenSpecMessage::Checking));
        assert_eq!(openspec_message(true, 2, 1, 0, 0),
                   Some(OpenSpecMessage::Checking));
    }

    /// "No changes" only once every repository has been scanned.
    #[test]
    fn no_changes_waits_for_every_scan() {
        assert_eq!(openspec_message(true, 2, 2, 0, 0),
                   Some(OpenSpecMessage::NoChanges));
        assert_eq!(openspec_message(true, 0, 0, 0, 0),
                   Some(OpenSpecMessage::NoChanges));
    }

    #[test]
    fn a_search_hiding_everything_says_nothing_matches() {
        assert_eq!(openspec_message(true, 2, 2, 3, 0),
                   Some(OpenSpecMessage::NoneMatch));
    }

    /// Rows already scanned show while another repository is still scanning.
    #[test]
    fn partial_answers_draw_what_they_have() {
        assert_eq!(openspec_message(true, 2, 1, 3, 3), None);
    }
}
