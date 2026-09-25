//! The Pull Requests pane: this workspace's recorded pull requests, one row
//! per pull request, grouped by the agents that opened them, newest first.
//!
//! A content-area takeover like the dashboard, not a popover on each agent
//! card: the question it answers is "what did this session produce", which is
//! a cross-agent question, and a per-card popover buries exactly that.

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use knot_forge::{CheckRollup, ForgeAvailability, Mergeability, PullRequestStatus};
use uuid::Uuid;

use crate::consts;
use crate::pull_request_state::PullRequestLookup;
use crate::workspace_window::WorkspaceWindow;

/// One row, flattened out of the store and the state cache before any
/// element is built - so neither lock is held across the element tree.
pub(crate) struct PullRequestRow {
    pub(crate) url:    String,
    /// `None` while nothing has been asked for it yet.
    pub(crate) lookup: Option<PullRequestLookup>,
}

/// What a row can say about where its pull request stands.
///
/// Derived once from the lookup, so the icon and the detail line cannot
/// disagree. A failed fetch reads as pending: it is retried, so "checking" is
/// still true of it. Not found is not: the answer is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowStatus {
    Known(PullRequestStatus),
    NotFound,
    Pending,
}

impl RowStatus {
    fn of(lookup: Option<&PullRequestLookup>) -> Self {
        match lookup {
            Some(PullRequestLookup::Known(state)) => Self::Known(state.status),
            Some(PullRequestLookup::NotFound) => Self::NotFound,
            Some(PullRequestLookup::Failed) | None => Self::Pending,
        }
    }

    fn known(self) -> Option<PullRequestStatus> {
        match self {
            Self::Known(status) => Some(status),
            Self::NotFound | Self::Pending => None,
        }
    }
}

/// The rows one set of agents opened, under their names.
///
/// Several agents rather than one: a pull request several agents recorded is
/// one row, headed by all of them.
pub(crate) struct PullRequestGroup {
    /// Every agent each row here is attributed to, so removing a row
    /// addresses every record behind it.
    pub(crate) agent_ids: Vec<Uuid>,
    /// Their names, joined for the heading.
    pub(crate) agents:    String,
    pub(crate) rows:      Vec<PullRequestRow>,
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
        forge_notice_text(&self.forge_status.availability()?)
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
    let agent_ids = group.agent_ids;
    v_flex().gap_2()
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(knot_core::l10n::t_with("pull_requests.opened_by",
                                                       &[("name", &group.agents)])))
            .children(group.rows
                           .into_iter()
                           .map(|row| render_row(agent_ids.clone(), row, cx)))
            .into_any_element()
}

fn render_row(agent_ids: Vec<Uuid>, row: PullRequestRow, cx: &mut Context<WorkspaceWindow>)
              -> gpui_kit::AnyElement {
    let url = row.url.clone();
    let remove_url = row.url.clone();
    let state = row.lookup.as_ref().and_then(PullRequestLookup::state);
    let title = state.and_then(|state| state.title.clone())
                     .unwrap_or_else(|| row.url.clone());
    let number = state.and_then(|state| state.number)
                      .map(|number| format!("#{number}"));
    let status = RowStatus::of(row.lookup.as_ref());
    let checks = state.and_then(|state| state.checks);
    let mergeable = state.map_or(Mergeability::Unknown, |state| state.mergeable);
    let tint = state_color(status.known(), mergeable);

    h_flex().id(gpui_kit::SharedString::from(format!("pull-request-{}", row.url)))
            .w_full()
            .min_w_0()
            .gap_3()
            .items_center()
            .p_2()
            .rounded(cx.theme().radius)
            .border_1()
            // A row with no state earns no colour: the theme's own border and
            // no fill, so an uncoloured row reads as "not known yet" rather
            // than as a fifth state.
            .border_color(tint.map_or_else(|| cx.theme().border,
                                           |color| {
                                               color.opacity(consts::PULL_REQUEST_ROW_BORDER_TINT)
                                           }))
            .when_some(tint, |row, color| {
                row.bg(color.opacity(consts::PULL_REQUEST_ROW_TINT))
            })
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
                        // A trash can, not an ✕. Every other icon in this row
                        // is a *status*, so an ✕ among them reads as "failed"
                        // rather than as a control - and beside a pull request
                        // it reads as "close this pull request", which is the
                        // one thing Knot will never do. The settings window's
                        // delete buttons use the same icon.
                        .child(Icon::default().path(REMOVE_ICON))
                        // Still tooltipped: the icon says "delete", and the
                        // tooltip says delete from *what*, which is the part
                        // that matters when the forge is one click away.
                        .tooltip(|window, cx| {
                            Tooltip::new(knot_core::l10n::t("pull_requests.remove")).build(window,
                                                                                           cx)
                        })
                        .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                        // Before the removal, not after: gpui
                                        // fires click handlers in the bubble
                                        // phase, so without this the same click
                                        // goes on to the row's own handler and
                                        // opens the pull request the user was
                                        // removing. This is the only control in
                                        // `render/` nested inside a clickable
                                        // row, which is why nothing else needs
                                        // it. The dispatch order it relies on
                                        // is pinned by the row-click tests.
                                        cx.stop_propagation();
                                        view.confirm_remove_pull_request(agent_ids.clone(),
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

/// The colour a row wears, or `None` when no state has been fetched.
///
/// Purple merged, red closed; an open one says why it can or cannot land -
/// green mergeable, yellow behind its base, blue checks running, orange
/// conflicting or otherwise blocked - so the shape of a session's output is
/// readable without reading a word of it. An open pull request whose
/// mergeability GitHub has not computed yet gets no colour rather than an
/// optimistic green.
///
/// Orange covers both a conflict and every other block (draft, red CI, a
/// required review): each needs a push or a decision before it can land,
/// which is the one thing the colour has to say.
fn state_color(status: Option<PullRequestStatus>, mergeable: Mergeability)
               -> Option<gpui_kit::Hsla> {
    let color = match status? {
        PullRequestStatus::Merged => consts::COLOR_PULL_REQUEST_MERGED,
        PullRequestStatus::Closed => consts::COLOR_ERROR,
        PullRequestStatus::Draft | PullRequestStatus::Open => match mergeable {
            Mergeability::Mergeable => consts::COLOR_IDLE,
            Mergeability::Conflicting | Mergeability::Blocked => consts::COLOR_RUNNING,
            Mergeability::Behind => consts::COLOR_PULL_REQUEST_BEHIND,
            Mergeability::ChecksRunning => consts::COLOR_INPUT,
            Mergeability::Unknown => return None,
        },
    };
    Some(gpui_kit::rgb(color).into())
}

/// The icon on the row's remove control.
///
/// A trash can rather than an X: every other icon in the row is a status, so
/// an X among them reads as "failed" rather than as a control - and beside a
/// pull request it reads as "close this pull request", the one thing Knot
/// will never do. The settings window's delete buttons use the same icon.
pub(super) const REMOVE_ICON: &str = "icons/trash.svg";

/// The icon for a row's state. Draft has its own, because the whole point of
/// a draft is that it is not ready - which a plain open icon does not say.
fn status_icon(status: RowStatus) -> &'static str {
    match status {
        RowStatus::Known(PullRequestStatus::Draft) => "icons/git-pull-request-draft.svg",
        RowStatus::Known(PullRequestStatus::Merged) => "icons/git-merge.svg",
        RowStatus::Known(PullRequestStatus::Closed) => "icons/git-pull-request-closed.svg",
        RowStatus::Known(PullRequestStatus::Open) => "icons/git-pull-request.svg",
        // Looked for and not there. Not a pull request icon, since the forge
        // says there is no pull request, and not an error icon, since
        // nothing failed.
        RowStatus::NotFound => "icons/search-x.svg",
        // No state fetched: the row still lists and still opens, so it gets
        // the neutral icon rather than one that claims a state.
        RowStatus::Pending => "icons/git-pull-request-arrow.svg",
    }
}

/// The second line: number, state and checks, with whatever is absent simply
/// left out rather than shown as a blank.
///
/// Checks only while the pull request is open. Once it is merged or closed
/// they say nothing anyone will act on.
fn detail_line(number: Option<&str>, status: RowStatus, checks: Option<CheckRollup>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(number) = number {
        parts.push(number.to_string());
    }
    parts.push(match status {
                   RowStatus::Known(PullRequestStatus::Draft) => {
                       knot_core::l10n::t("pull_requests.draft")
                   }
                   RowStatus::Known(PullRequestStatus::Open) => {
                       knot_core::l10n::t("pull_requests.open")
                   }
                   RowStatus::Known(PullRequestStatus::Merged) => {
                       knot_core::l10n::t("pull_requests.merged")
                   }
                   RowStatus::Known(PullRequestStatus::Closed) => {
                       knot_core::l10n::t("pull_requests.closed")
                   }
                   RowStatus::NotFound => knot_core::l10n::t("pull_requests.not_found"),
                   RowStatus::Pending => knot_core::l10n::t("pull_requests.pending"),
               });
    if let Some(checks) = checks.filter(|_| status.known().is_some_and(PullRequestStatus::is_open))
    {
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
