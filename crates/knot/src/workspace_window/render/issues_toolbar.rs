//! The Issues tab's toolbar (#504): search, repository picker, sort picker
//! and list actions.
//!
//! Contract: `workspace-issues`, "The user can search, filter and sort the
//! Issues tab". Drawn from the frame the pane computed, so Copy URLs copies
//! the rows on screen.

use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::{App, Context, IntoElement, ParentElement, Styled, Window, px};
use knot_forge::RepoSlug;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::issue_filter::{IssueGroup, IssueSort};

/// The search field's width, as the Pull Requests tab's.
const SEARCH_WIDTH: f32 = 300.;

impl WorkspaceWindow {
    pub(super) fn issues_toolbar(&mut self, slugs: Vec<RepoSlug>, shown: &[IssueGroup],
                                 window: &mut Window, cx: &mut Context<Self>)
                                 -> gpui_kit::AnyElement {
        let search = self.issues_search(window, cx);
        let urls = shown.iter()
                        .flat_map(|group| group.issues.iter().map(|issue| issue.url.clone()))
                        .collect::<Vec<_>>()
                        .join("\n");
        h_flex().gap_2()
                .items_center()
                .child(Input::new(&search).small()
                                          .cleanable(true)
                                          .border_color(cx.theme().border)
                                          .w(px(SEARCH_WIDTH)))
                .child(self.issues_repo_picker(slugs, cx))
                .child(self.issues_sort_picker(cx))
                .child(issues_actions_menu(urls, cx))
                .into_any_element()
    }

    fn issues_repo_picker(&self, slugs: Vec<RepoSlug>, cx: &mut Context<Self>) -> impl IntoElement {
        let all = knot_core::l10n::t("changes_view.repo_all");
        let current = self.changes_view
                          .issues
                          .repo
                          .as_ref()
                          .map_or_else(|| all.clone(), ToString::to_string);
        let entity = cx.entity();
        Button::new("issues-repo-picker").label(current)
                                         .dropdown_caret(true)
                                         .ghost()
                                         .small()
                                         .dropdown_menu(move |mut menu, _, _| {
                                             let choices =
                                                 std::iter::once((None, all.clone()))
                                                     .chain(slugs.iter()
                                                                 .map(|slug| {
                                                                     (Some(slug.clone()),
                                                                      slug.to_string())
                                                                 }));
                                             for (repo, label) in choices {
                                                 let entity = entity.clone();
                                                 menu = menu.item(PopupMenuItem::new(label)
                                                     .on_click(move |_, _, app| {
                                                         let repo = repo.clone();
                                                         entity.update(app, |view, cx| {
                                                             view.changes_view.issues.repo = repo;
                                                             cx.notify();
                                                         });
                                                     }));
                                             }
                                             menu
                                         })
    }

    fn issues_sort_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        Button::new("issues-sort-picker").label(self.changes_view.issues.sort.label())
                                         .dropdown_caret(true)
                                         .ghost()
                                         .small()
                                         .dropdown_menu(move |mut menu, _, _| {
                                             for sort in IssueSort::ALL {
                                                 let entity = entity.clone();
                                                 menu = menu.item(PopupMenuItem::new(sort.label())
                                                     .on_click(move |_, _, app| {
                                                         entity.update(app, |view, cx| {
                                                             view.changes_view.issues.sort = sort;
                                                             cx.notify();
                                                         });
                                                     }));
                                             }
                                             menu
                                         })
    }
}

/// Refresh now and Copy URLs. Refresh now also re-probes `gh`, so signing in
/// takes effect without leaving the tab.
fn issues_actions_menu(urls: String, cx: &mut Context<WorkspaceWindow>) -> impl IntoElement {
    let entity = cx.entity();
    crate::controls::icon_button("issues-actions",
                                 "icons/ellipsis.svg",
                                 knot_core::l10n::t("changes_view.list_actions"),
                                 false).dropdown_menu(move |menu, _, _| {
        let refresh = entity.clone();
        let text = urls.clone();
        menu.item(PopupMenuItem::new(knot_core::l10n::t("changes_view.refresh_now")).on_click(
                      move |_, _, app| {
                          refresh.update(app, |view, cx| {
                                     view.refresh_issues_now();
                                     cx.notify();
                                 });
                      },
                  ))
            .item(PopupMenuItem::new(knot_core::l10n::t("issues.copy_urls"))
                      .disabled(text.is_empty())
                      .on_click(move |_, _, app: &mut App| {
                          WorkspaceWindow::copy_to_clipboard(text.clone(), app);
                      }))
    })
}
