//! The Changes view's frame: its title, the tab bar, and the chosen tab's
//! body below them (#504).
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! pull-request-tracking/spec.md`, "The Changes view is divided into tabs".
//! Each tab's body is its own module - `pull_requests_pane`, `issues_pane`,
//! `openspec_pane` - and keeps its own search and filters, so switching tabs
//! neither clears nor shares them.

use gpui_kit::base::{StyledExt, v_flex};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, Window, div};

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::changes_tab::ChangesTab;

impl WorkspaceWindow {
    /// The Changes view, or `None` when the window is not showing it.
    pub(super) fn changes_content(&mut self, is_showing: bool, window: &mut Window,
                                  cx: &mut Context<Self>)
                                  -> Option<gpui_kit::AnyElement> {
        if !is_showing {
            return None;
        }
        let tab = self.changes_view.tab;
        let body = match tab {
            ChangesTab::PullRequests => self.pull_requests_body(window, cx),
            ChangesTab::Issues => self.issues_body(window, cx),
            ChangesTab::OpenSpec => self.openspec_body(window, cx),
        };
        Some(v_flex().flex_1()
                     .min_h_0()
                     .child(v_flex().flex_shrink_0()
                                    .gap_2()
                                    .px_5()
                                    .pt_5()
                                    .child(div().text_lg()
                                                .font_semibold()
                                                .child(knot_core::l10n::t("changes_view.title")))
                                    .child(self.changes_tab_bar(tab, cx)))
                     .child(body)
                     .into_any_element())
    }

    fn changes_tab_bar(&self, selected: ChangesTab, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        TabBar::new("changes-tabs").underline()
                                   .selected_index(selected.index())
                                   .children(ChangesTab::ALL.map(|tab| {
                                                                Tab::new().label(tab.label())
                                                            }))
                                   .on_click(move |index, _, app| {
                                       let Some(&tab) = ChangesTab::ALL.get(*index)
                                       else {
                                           return;
                                       };
                                       entity.update(app, |view, cx| {
                                                 view.changes_view.tab = tab;
                                                 cx.notify();
                                             });
                                   })
    }
}
