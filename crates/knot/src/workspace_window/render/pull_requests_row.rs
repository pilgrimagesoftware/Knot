//! The Pull Requests launcher row in the sidebar.
//!
//! Shaped like the dashboard row above it, for the same reason that one is
//! not an icon in the bottom bar: it is a view of the whole workspace, so it
//! belongs with the rows it summarises.
//!
//! What it adds is the breakdown. The count is the point of the row - "what
//! did this session produce, and where does it stand" - so it shows open,
//! merged and closed rather than one total the user has to open the view to
//! interpret.

use gpui_kit::base::{StyledExt, h_flex};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};

use crate::pull_request_state::PullRequestCounts;
use crate::workspace_window::{WorkspaceViewMode, WorkspaceWindow};

impl WorkspaceWindow {
    /// The Pull Requests row, or `None` when this workspace has none
    /// recorded.
    ///
    /// Absent rather than empty: a workspace whose agents have never opened a
    /// pull request has nothing the row could lead to, and a permanent "0"
    /// above the agent list is a row that never does anything.
    pub(super) fn pull_requests_row(&self, compact: bool, cx: &mut Context<Self>)
                                    -> Option<gpui_kit::AnyElement> {
        let counts = self.pull_request_counts();
        if counts.total() == 0 {
            return None;
        }
        let is_showing = self.view_mode == WorkspaceViewMode::PullRequests;

        // The second line ellipsizes in a narrow sidebar, and the compact
        // sidebar drops it altogether - so the whole of it goes in a tooltip,
        // the one place the full breakdown is always readable.
        let tooltip = format!("{} — {}",
                              knot_core::l10n::t("pull_requests.title"),
                              counts_label(counts));

        Some(div().id("workspace-pull-requests-row")
                  .cursor_pointer()
                  .rounded(cx.theme().radius)
                  .p_2()
                  .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                  .bg(if is_showing {
                      cx.theme().muted
                  }
                  else {
                      cx.theme().transparent
                  })
                  .child(h_flex().w_full()
                                 .gap_3()
                                 .items_center()
                                 .when(compact, |row| row.justify_center())
                                 .child(div().w(px(40.))
                                             .h(px(40.))
                                             .flex_shrink_0()
                                             .flex()
                                             .items_center()
                                             .justify_center()
                                             .child(Icon::default().path("icons/git-pull-request.svg")))
                                 .when(!compact, |row| {
                                     // `min_w_0` on the growing child, not
                                     // just `flex_1`: the breakdown is the
                                     // longest text in the sidebar, and
                                     // without it the row pushes the divider
                                     // instead of ellipsizing.
                                     row.child(div().flex_1()
                                                    .min_w_0()
                                                    .child(title_and_counts(counts, cx)))
                                 }))
                  .on_click(cx.listener(|view, _: &ClickEvent, _window, cx| {
                                view.view_mode =
                                    view.view_mode
                                        .toggled(WorkspaceViewMode::PullRequests);
                                cx.notify();
                            }))
                  .into_any_element())
    }
}

/// The row's label and, under it, what is known about the states.
fn title_and_counts(counts: PullRequestCounts, cx: &mut Context<WorkspaceWindow>)
                    -> gpui_kit::AnyElement {
    gpui_kit::base::v_flex().min_w_0()
                            .child(div().font_semibold()
                                        .child(knot_core::l10n::t("pull_requests.title")))
                            .child(div().text_xs()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(counts_label(counts)))
                            .into_any_element()
}

/// What the row says about the states it knows.
///
/// A breakdown once anything has been fetched, and the plain total before
/// that. Showing "0 open · 0 merged · 0 closed" for four records Knot has not
/// asked about yet would state three things it does not know; "4" states the
/// one thing it does.
///
/// A state nothing is in is left out entirely. "0 closed" is a fact nobody
/// needs and a third of the row's width, and dropping it makes the counts
/// that are there easier to read.
///
/// Assembled from per-count entries rather than one sentence, which the
/// localization rule would normally forbid: this is a list of labelled
/// numbers whose membership varies, not a sentence with a value in it, so
/// each part stays independently translatable and the separator is
/// punctuation.
pub(crate) fn counts_label(counts: PullRequestCounts) -> String {
    if counts.nothing_known() {
        return knot_core::l10n::t_with("pull_requests.counts_total",
                                       &[("count", &counts.total().to_string())]);
    }
    [("pull_requests.count_open", counts.open),
     ("pull_requests.count_merged", counts.merged),
     ("pull_requests.count_closed", counts.closed),
     ("pull_requests.count_pending", counts.pending)].into_iter()
                                                     .filter(|(_, count)| *count > 0)
                                                     .map(|(key, count)| {
                                                         let count = count.to_string();
                                                         knot_core::l10n::t_with(key,
                                                                                 &[("count",
                                                                                    &count)])
                                                     })
                                                     .collect::<Vec<_>>()
                                                     .join(" · ")
}

#[cfg(test)]
mod tests;
