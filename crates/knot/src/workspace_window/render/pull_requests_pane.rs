//! The Pull Requests pane: this workspace's recorded pull requests, grouped
//! by the agent that opened them, newest first.
//!
//! A content-area takeover like the dashboard, not a popover on each agent
//! card: the question it answers is "what did this session produce", which is
//! a cross-agent question, and a per-card popover buries exactly that.

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use knot_forge::{CheckRollup, ForgeAvailability, PullRequestState, PullRequestStatus};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// One row, flattened out of the store and the state cache before any
/// element is built - so neither lock is held across the element tree.
pub(crate) struct PullRequestRow {
    pub(crate) url:   String,
    pub(crate) state: Option<PullRequestState>,
}

/// One agent's rows, under that agent's name.
pub(crate) struct PullRequestGroup {
    pub(crate) agent_id: Uuid,
    pub(crate) agent:    String,
    pub(crate) rows:     Vec<PullRequestRow>,
}

impl WorkspaceWindow {
    /// The Pull Requests pane, or `None` when the window is not showing it.
    pub(super) fn pull_requests_content(&mut self, is_showing: bool, cx: &mut Context<Self>)
                                        -> Option<gpui_kit::AnyElement> {
        if !is_showing {
            return None;
        }
        let groups = self.pull_request_groups();
        let notice = self.forge_notice();

        Some(div().id("workspace-pull-requests")
                  .flex_1()
                  .min_h_0()
                  .overflow_y_scroll()
                  .child(v_flex().min_h_full()
                                 .gap_4()
                                 .p_5()
                                 .child(div().text_lg()
                                             .font_semibold()
                                             .child(knot_core::l10n::t("pull_requests.title")))
                                 .children(notice.map(|text| {
                                                     div().text_sm()
                                                          .text_color(cx.theme().muted_foreground)
                                                          .child(text)
                                                 }))
                                 // A click that opened nothing has to say so, or the row
                                 // reads as broken rather than as a browser that refused.
                                 .children(self.pull_request_open_failed.then(|| {
                                                                            div().text_sm()
                                          .text_color(cx.theme().danger)
                                          .child(knot_core::l10n::t("pull_requests.open_failed"))
                                                                        }))
                                 .children(groups.is_empty().then(|| {
                                                                div().text_sm()
                                          .text_color(cx.theme().muted_foreground)
                                          .child(knot_core::l10n::t("pull_requests.empty"))
                                                            }))
                                 .children(groups.into_iter().map(|group| render_group(group, cx))))
                  .into_any_element())
    }

    /// The one availability message for the whole view, or `None` when state
    /// is being read normally.
    ///
    /// Once for the view rather than once per row: twenty rows saying `gh` is
    /// not installed is twenty copies of one fact, and the fix is the same
    /// for all of them.
    fn forge_notice(&self) -> Option<String> {
        forge_notice_text(self.forge_status.availability()?)
    }
}

/// The message for one availability, or `None` when there is nothing to say.
///
/// The three failures stay apart because they have different fixes: install
/// `gh`, sign in, or look at why the request failed. Collapsing them into one
/// "unavailable" leaves the user without the next step.
fn forge_notice_text(availability: &ForgeAvailability) -> Option<String> {
    match availability {
        ForgeAvailability::Ready => None,
        ForgeAvailability::Missing => Some(knot_core::l10n::t("pull_requests.forge_missing")),
        ForgeAvailability::Unauthenticated => {
            Some(knot_core::l10n::t("pull_requests.forge_unauthenticated"))
        }
        ForgeAvailability::Failed(reason) => {
            Some(knot_core::l10n::t_with("pull_requests.forge_failed", &[("reason", reason)]))
        }
    }
}

fn render_group(group: PullRequestGroup, cx: &mut Context<WorkspaceWindow>)
                -> gpui_kit::AnyElement {
    let agent_id = group.agent_id;
    v_flex().gap_2()
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(knot_core::l10n::t_with("pull_requests.opened_by",
                                                       &[("name", &group.agent)])))
            .children(group.rows
                           .into_iter()
                           .map(|row| render_row(agent_id, row, cx)))
            .into_any_element()
}

fn render_row(agent_id: Uuid, row: PullRequestRow, cx: &mut Context<WorkspaceWindow>)
              -> gpui_kit::AnyElement {
    let url = row.url.clone();
    let remove_url = row.url.clone();
    let title = row.state
                   .as_ref()
                   .and_then(|state| state.title.clone())
                   .unwrap_or_else(|| row.url.clone());
    let number = row.state
                    .as_ref()
                    .and_then(|state| state.number)
                    .map(|number| format!("#{number}"));
    let status = row.state.as_ref().map(|state| state.status);
    let checks = row.state.as_ref().and_then(|state| state.checks);

    h_flex().id(gpui_kit::SharedString::from(format!("pull-request-{}", row.url)))
            .w_full()
            .min_w_0()
            .gap_3()
            .items_center()
            .p_2()
            .rounded(cx.theme().radius)
            .cursor_pointer()
            .hover(|row| row.bg(cx.theme().muted))
            .child(div().flex_shrink_0()
                        .child(Icon::default().path(status_icon(status))))
            // `min_w_0` on the growing child: a long pull request title must
            // ellipsize inside the pane rather than stretch it.
            .child(v_flex().flex_1()
                           .min_w_0()
                           .child(div().overflow_hidden()
                                       .whitespace_nowrap()
                                       .text_ellipsis()
                                       .child(title))
                           .child(div().text_xs()
                                       .text_color(cx.theme().muted_foreground)
                                       .child(detail_line(number.as_deref(), status, checks))))
            .child(div().id(gpui_kit::SharedString::from(format!("remove-{}", row.url)))
                        .flex_shrink_0()
                        .px_2()
                        .cursor_pointer()
                        .text_color(cx.theme().danger)
                        .child(Icon::default().path("icons/x.svg"))
                        .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                        view.confirm_remove_pull_request(agent_id,
                                                                         remove_url.clone(),
                                                                         window,
                                                                         cx);
                                    })))
            .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                            view.open_pull_request(&url);
                            cx.notify();
                        }))
            .into_any_element()
}

/// The icon for a row's state. Draft has its own, because the whole point of
/// a draft is that it is not ready - which a plain open icon does not say.
fn status_icon(status: Option<PullRequestStatus>) -> &'static str {
    match status {
        Some(PullRequestStatus::Draft) => "icons/git-pull-request-draft.svg",
        Some(PullRequestStatus::Merged) => "icons/git-merge.svg",
        Some(PullRequestStatus::Closed) => "icons/git-pull-request-closed.svg",
        Some(PullRequestStatus::Open) => "icons/git-pull-request.svg",
        // No state fetched: the row still lists and still opens, so it gets
        // the neutral icon rather than one that claims a state.
        None => "icons/git-pull-request-arrow.svg",
    }
}

/// The second line: number, state and checks, with whatever is absent simply
/// left out rather than shown as a blank.
fn detail_line(number: Option<&str>, status: Option<PullRequestStatus>,
               checks: Option<CheckRollup>)
               -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(number) = number {
        parts.push(number.to_string());
    }
    parts.push(match status {
                   Some(PullRequestStatus::Draft) => knot_core::l10n::t("pull_requests.draft"),
                   Some(PullRequestStatus::Open) => knot_core::l10n::t("pull_requests.open"),
                   Some(PullRequestStatus::Merged) => knot_core::l10n::t("pull_requests.merged"),
                   Some(PullRequestStatus::Closed) => knot_core::l10n::t("pull_requests.closed"),
                   None => knot_core::l10n::t("pull_requests.pending"),
               });
    if let Some(checks) = checks {
        parts.push(match checks {
                       CheckRollup::Passing => knot_core::l10n::t("pull_requests.checks_passing"),
                       CheckRollup::Failing => knot_core::l10n::t("pull_requests.checks_failing"),
                       CheckRollup::Pending => knot_core::l10n::t("pull_requests.checks_pending"),
                   });
    }
    parts.join(" · ")
}

#[cfg(test)]
mod tests;
