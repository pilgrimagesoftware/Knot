//! The OpenSpec tab's toolbar (#504): search, repository picker and list
//! actions.
//!
//! Contract: `workspace-openspec-changes`, "The user can search and filter
//! the OpenSpec tab".

use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, Window, px};

use crate::workspace_window::WorkspaceWindow;

/// The search field's width, as the other tabs'.
const SEARCH_WIDTH: f32 = 300.;

impl WorkspaceWindow {
    pub(super) fn openspec_toolbar(&mut self, headings: Vec<String>, window: &mut Window,
                                   cx: &mut Context<Self>)
                                   -> gpui_kit::AnyElement {
        let search = self.openspec_search(window, cx);
        let entity = cx.entity();
        h_flex().gap_2()
                .items_center()
                .child(Input::new(&search).small()
                                          .cleanable(true)
                                          .border_color(cx.theme().border)
                                          .w(px(SEARCH_WIDTH)))
                .child(self.openspec_repo_picker(headings, cx))
                .child(crate::controls::icon_button("openspec-actions",
                                                    "icons/ellipsis.svg",
                                                    knot_core::l10n::t("changes_view.list_actions"),
                                                    false).dropdown_menu(move |menu, _, _| {
                    let refresh = entity.clone();
                    menu.item(PopupMenuItem::new(knot_core::l10n::t("changes_view.refresh_now"))
                                  .on_click(move |_, _, app| {
                                      refresh.update(app, |view, cx| {
                                                 view.refresh_openspec_now();
                                                 cx.notify();
                                             });
                                  }))
                }))
                .into_any_element()
    }

    fn openspec_repo_picker(&self, headings: Vec<String>, cx: &mut Context<Self>)
                            -> impl IntoElement {
        let all = knot_core::l10n::t("changes_view.repo_all");
        let current = self.changes_view
                          .openspec
                          .repo
                          .clone()
                          .unwrap_or_else(|| all.clone());
        let entity = cx.entity();
        Button::new("openspec-repo-picker").label(current)
                                           .dropdown_caret(true)
                                           .ghost()
                                           .small()
                                           .dropdown_menu(move |mut menu, _, _| {
                                               let choices =
                                                   std::iter::once((None, all.clone()))
                                                       .chain(headings.iter()
                                                                      .map(|heading| {
                                                                          (Some(heading.clone()),
                                                                           heading.clone())
                                                                      }));
                                               for (repo, label) in choices {
                                                   let entity = entity.clone();
                                                   menu = menu.item(PopupMenuItem::new(label)
                                                       .on_click(move |_, _, app| {
                                                           let repo = repo.clone();
                                                           entity.update(app, |view, cx| {
                                                               view.changes_view.openspec.repo =
                                                                   repo;
                                                               cx.notify();
                                                           });
                                                       }));
                                               }
                                               menu
                                           })
    }
}
