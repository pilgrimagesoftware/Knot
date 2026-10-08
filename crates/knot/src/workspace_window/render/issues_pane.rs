//! The Changes view's Issues tab (#504): the workspace's repositories' open
//! issues, grouped by repository.
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! workspace-issues/spec.md`. What a frame shows is decided in
//! `workspace_window::issues_view`; this draws it.

use std::time::SystemTime;

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    App, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div,
};
use knot_forge::{Issue, RepoSlug};

use super::row_actions::row_actions_button;
use super::send_prompt_menu::{SendTarget, WorkItemRef, with_send_prompt};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::issue_filter::IssueGroup;
use crate::workspace_window::issues_view::IssuesMessage;

/// One repository's heading and what sits under it this frame.
enum RepoSection {
    Issues(IssueGroup),
    Failed(RepoSlug, String),
}

impl RepoSection {
    fn slug(&self) -> &RepoSlug {
        match self {
            Self::Issues(group) => &group.slug,
            Self::Failed(slug, _) => slug,
        }
    }
}

impl WorkspaceWindow {
    /// The Issues tab's body.
    pub(super) fn issues_body(&mut self, window: &mut Window, cx: &mut Context<Self>)
                              -> gpui_kit::AnyElement {
        let frame = self.issues_frame(cx);
        let toolbar = self.issues_toolbar(frame.slugs.clone(), &frame.shown, window, cx);
        let targets = self.prompt_targets();
        let mut sections: Vec<RepoSection> =
            frame.shown
                 .into_iter()
                 .map(RepoSection::Issues)
                 .chain(frame.failed
                             .into_iter()
                             .map(|(slug, reason)| RepoSection::Failed(slug, reason)))
                 .collect();
        sections.sort_by(|a, b| a.slug().cmp(b.slug()));
        let entity = cx.entity();

        div().id("workspace-issues")
             .flex_1()
             .min_h_0()
             .overflow_y_scrollbar()
             .child(v_flex().min_h_full()
                            .gap_4()
                            .p_5()
                            .child(toolbar)
                            .children(frame.message.map(|message| render_message(message, cx)))
                            .children(sections.into_iter().map(|section| {
                                                              let truncated =
                                                                  frame.truncated
                                                                       .contains(section.slug());
                                                              render_section(section, truncated,
                                                                             &targets, &entity, cx)
                                                          })))
             .into_any_element()
    }
}

/// The message standing in for the list. Every "nothing to show" says why,
/// and the one with a fix in the toolbar's reach - a search hiding
/// everything - offers it.
fn render_message(message: IssuesMessage, cx: &mut Context<WorkspaceWindow>)
                  -> gpui_kit::AnyElement {
    let muted = cx.theme().muted_foreground;
    let text = match &message {
        IssuesMessage::Checking => knot_core::l10n::t("issues.checking"),
        IssuesMessage::ForgeMissing => knot_core::l10n::t("issues.forge_missing"),
        IssuesMessage::ForgeUnauthenticated => knot_core::l10n::t("issues.forge_unauthenticated"),
        IssuesMessage::ForgeFailed(reason) => {
            knot_core::l10n::t_with("issues.forge_failed", &[("reason", reason)])
        }
        IssuesMessage::NoGithubRepos => knot_core::l10n::t("issues.no_github_repos"),
        IssuesMessage::NoneOpen => knot_core::l10n::t("issues.empty"),
        IssuesMessage::NoneMatch => knot_core::l10n::t("issues.no_match"),
    };
    v_flex().gap_2()
            .items_start()
            .child(div().text_sm().text_color(muted).child(text))
            .children((message == IssuesMessage::NoneMatch).then(|| {
                                                               Button::new("issues-clear-filters")
                    .label(knot_core::l10n::t("changes_view.clear_filters"))
                    .on_click(cx.listener(|view, _: &ClickEvent, window, cx| {
                                    view.clear_issue_filters(window, cx);
                                }))
                                                           }))
            .into_any_element()
}

fn render_section(section: RepoSection, truncated: bool, targets: &[SendTarget],
                  entity: &Entity<WorkspaceWindow>, cx: &mut Context<WorkspaceWindow>)
                  -> gpui_kit::AnyElement {
    let muted = cx.theme().muted_foreground;
    let heading = div().text_sm()
                       .font_semibold()
                       .child(section.slug().to_string());
    match section {
        RepoSection::Failed(slug, reason) => {
            v_flex().gap_1()
                    .child(heading)
                    .child(div().text_sm()
                                .text_color(cx.theme().danger)
                                .child(knot_core::l10n::t_with("issues.fetch_failed",
                                                               &[("repo", &slug.to_string()),
                                                                 ("reason", &reason)])))
                    .into_any_element()
        }
        RepoSection::Issues(group) => {
            v_flex().gap_2()
                    .child(heading)
                    .children(truncated.then(|| {
                                           div().text_xs().text_color(muted).child(
                            knot_core::l10n::t_with("issues.truncated",
                                                    &[("count",
                                                       &knot_forge::consts::ISSUE_LIST_LIMIT
                                                           .to_string())]),
                        )
                                       }))
                    .children(group.issues
                                   .into_iter()
                                   .map(|issue| render_issue(issue, targets, entity, cx)))
                    .into_any_element()
        }
    }
}

fn render_issue(issue: Issue, targets: &[SendTarget], entity: &Entity<WorkspaceWindow>,
                cx: &mut Context<WorkspaceWindow>)
                -> gpui_kit::AnyElement {
    let muted = cx.theme().muted_foreground;
    let url = issue.url.clone();
    let updated = crate::timestamp::relative_timestamp(SystemTime::from(issue.updated_at));
    let detail = std::iter::once(format!("#{}", issue.number))
                     .chain(std::iter::once(knot_core::l10n::t_with("issues.updated",
                                                                    &[("when", &updated)])))
                     .collect::<Vec<_>>()
                     .join(" · ");
    let menu = IssueMenu { url:     issue.url.clone(),
                           targets: targets.to_vec(),
                           entity:  entity.clone(), };
    let context = menu.clone();
    h_flex().id(SharedString::from(format!("issue-{}", issue.url)))
            .w_full()
            .min_w_0()
            .gap_3()
            .items_center()
            .p_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .cursor_pointer()
            .hover(|row| row.bg(cx.theme().muted))
            .child(v_flex().flex_1()
                           .min_w_0()
                           .child(div().overflow_hidden()
                                       .whitespace_nowrap()
                                       .text_ellipsis()
                                       .child(issue.title.clone()))
                           .child(h_flex().gap_2()
                                          .min_w_0()
                                          .text_xs()
                                          .text_color(muted)
                                          .child(detail)
                                          .children(issue.labels.iter().map(|label| {
                                                                         div().px_1()
                                                                              .rounded_sm()
                                                                              .border_1()
                                                                              .border_color(cx.theme().border)
                                                                              .child(label.clone())
                                                                     }))))
            .child(row_actions_button(SharedString::from(format!("issue-actions-{}", issue.url)),
                                      move |popup, window, cx| menu.fill(popup, window, cx)))
            .on_click(move |_: &ClickEvent, _, _| {
                open_issue(&url);
            })
            // Last: it wraps the row, so every handler above stays the row's.
            .context_menu(move |popup, window, cx| context.fill(popup, window, cx))
            .into_any_element()
}

/// An issue row's menu, the same in its context menu and its actions menu.
#[derive(Clone)]
struct IssueMenu {
    url:     String,
    targets: Vec<SendTarget>,
    entity:  Entity<WorkspaceWindow>,
}

impl IssueMenu {
    fn fill(&self, menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>) -> PopupMenu {
        let open = self.url.clone();
        let copied = self.url.clone();
        let menu = menu.item(PopupMenuItem::new(knot_core::l10n::t("issues.open_in_browser"))
                                 .on_click(move |_, _, _| open_issue(&open)))
                       .item(PopupMenuItem::new(knot_core::l10n::t("issues.copy_url"))
                                 .on_click(move |_, _, app: &mut App| {
                                     WorkspaceWindow::copy_to_clipboard(copied.clone(), app);
                                 }));
        with_send_prompt(menu,
                         WorkItemRef::Issue(self.url.clone()),
                         self.targets.clone(),
                         self.entity.clone(),
                         window,
                         cx)
    }
}

/// Opens `url` in the default browser. A failure is logged rather than
/// shown: the row's Copy URL is the way out, one click away.
fn open_issue(url: &str) {
    if !crate::open_in::open_url(url) {
        eprintln!("could not open {url} in a browser");
    }
}
