//! The window header: the selected agent's identity on the left, its state
//! and diff stat on the right - or, on the dashboard, the dashboard's title
//! and its sort picker.

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;

use crate::app_state::state_color;
use crate::app_state::state_label;
use crate::app_support::single_line;
use crate::dashboard;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::creation::SelectedAgentHeader;

impl WorkspaceWindow {
    /// The header's leading half.
    pub(super) fn title_bar_left(&self, is_dashboard: bool,
                                 selected_header: &Option<SelectedAgentHeader>,
                                 title_font_name: &str, title_font_size: gpui_kit::Pixels,
                                 cx: &mut Context<Self>)
                                 -> gpui_kit::AnyElement {
        let title_font_name = title_font_name.to_owned();
        // Matches the Swift reference's title bar: it shows the selected
        // agent's identity directly (not a separate workspace-name strip
        // above a second header row) so the header abuts the traffic
        // lights with no redundant band, and a right-hand state/git-stats
        // indicator (`AgentFullHeader`) when a non-shell agent is selected.
        if is_dashboard {
            div().text_lg()
                 .child(knot_core::l10n::t("dashboard.title"))
                 .into_any_element()
        }
        else {
            match &selected_header {
                Some(header) => {
                    // The avatar and name always stay whole; the folder and
                    // the agent's status line give up space and ellipsize,
                    // the status line first since it is the longest and the
                    // least identifying.
                    h_flex().flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_3()
                            .child(div().flex_shrink_0()
                                        .text_2xl()
                                        .child(header.avatar.clone()))
                            .child(div().flex_shrink_0()
                                        .text_lg()
                                        .font_semibold()
                                        .child(header.name.clone()))
                            .child(div().flex_shrink(1.)
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .font_family(title_font_name.clone())
                                        .text_size(title_font_size)
                                        .text_color(cx.theme().muted_foreground)
                                        .child(single_line(&header.folder)))
                            .when(!header.header_title.is_empty(), |row| {
                                row.child(div().flex_shrink_0()
                                               .font_family(title_font_name.clone())
                                               .text_size(title_font_size)
                                               .text_color(cx.theme().muted_foreground)
                                               .child("●"))
                                   .child(div().flex_1()
                                               .min_w_0()
                                               .overflow_hidden()
                                               .whitespace_nowrap()
                                               .text_ellipsis()
                                               .font_family(title_font_name.clone())
                                               .text_size(title_font_size)
                                               .text_color(cx.theme().muted_foreground)
                                               .child(single_line(&header.header_title)))
                            })
                            .into_any_element()
                }
                None => div().text_lg()
                             .text_color(cx.theme().muted_foreground)
                             .child(knot_core::l10n::t("workspace.choose_agent"))
                             .into_any_element(),
            }
        }
    }

    /// The header's trailing half.
    pub(super) fn title_bar_right(&self, is_dashboard: bool,
                                  selected_header: &Option<SelectedAgentHeader>,
                                  title_font_name: &str, title_font_size: gpui_kit::Pixels,
                                  cx: &mut Context<Self>)
                                  -> gpui_kit::AnyElement {
        let title_font_name = title_font_name.to_owned();
        let weak = cx.entity().downgrade();
        if is_dashboard {
            dashboard::sort_picker(self.dashboard_sort, {
                let weak = weak.clone();
                move |sort, _window, app| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |view, cx| {
                                  view.dashboard_sort = sort;
                                  cx.notify();
                              });
                    }
                }
            }).into_any_element()
        }
        else {
            match selected_header.as_ref()
                                 .and_then(|header| header.state.as_ref())
            {
                Some((state, git_stats)) => {
                    v_flex().items_end()
                            .gap_0p5()
                            .child(h_flex().items_center()
                                           .gap_2()
                                           .child(div().w(px(10.))
                                                       .h(px(10.))
                                                       .rounded_full()
                                                       .bg(state_color(*state)))
                                           .child(div().font_family(title_font_name.clone())
                                                       .text_size(title_font_size)
                                                       .text_color(cx.theme().muted_foreground)
                                                       .child(state_label(*state))))
                            .child(match git_stats {
                                       Some(Some(stats)) => {
                                           // The stat row is already the
                                           // "this folder has N changes"
                                           // indicator, so it is what opens
                                           // the panel that shows what those
                                           // changes are - the question the
                                           // row raises and cannot answer.
                                           Self::render_diff_stats_button(stats,
                                                                          title_font_name.clone(),
                                                                          title_font_size,
                                                                          cx)
                                       }
                                       // The refresh ran and found no
                                       // repository: the agent's folder
                                       // isn't a git checkout, so there
                                       // are no stats to wait for.
                                       Some(None) => div().into_any_element(),
                                       None => div().font_family(title_font_name.clone())
                                                    .text_size(title_font_size)
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(knot_core::l10n::t("git.stats_pending"))
                                                    .into_any_element(),
                                   })
                            .into_any_element()
                }
                None => div().into_any_element(),
            }
        }
    }
}
