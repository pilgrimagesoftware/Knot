//! The sidebar column: the dashboard row that sits above the list, and the
//! agent rows themselves.
//!
//! An agent row is built from an [`AgentRow`] snapshot rather than from the
//! store directly, so the store lock is released before any element is
//! built - see [`super`]'s `render`.

use std::path::PathBuf;
use std::sync::Arc;

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;
use unicode_segmentation::UnicodeSegmentation;

use super::sidebar_compact::{CompactAgentRow, compact_agent_row_body};
use crate::app_state::state_color;
use crate::app_support::single_line;
use crate::consts;
use crate::settings_window::SettingsWindow;
use crate::workspace_window::AgentMenuTargets;
use crate::workspace_window::AgentRow;
use crate::workspace_window::DetailLineSize;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::agent_row_context_menu;
use crate::workspace_window::detail_line;

impl WorkspaceWindow {
    /// One element per agent in this workspace, in sidebar order.
    ///
    /// Collected rather than returned lazily because the closures borrow
    /// `cx`, which the caller needs back to build the rest of the tree.
    /// `compact` swaps each row's *body* for the avatar-only one; everything
    /// around the body - selection, dimming, the companion indent, the click
    /// handler and the context menu - is wired once and applies either way.
    pub(super) fn agent_rows(&mut self, agents: Vec<AgentRow>, title_font_name: String,
                             title_font_size: gpui_kit::Pixels, compact: bool,
                             cx: &mut Context<Self>)
                             -> Vec<gpui_kit::AnyElement> {
        let store_for_menu = Arc::clone(&self.store);
        let settings_for_menu = self.settings.clone();
        let workspace_id = self.workspace_id;
        let window_entity = cx.entity();

        agents.into_iter()
              .map(|AgentRow { id,
                               avatar,
                               name,
                               folder,
                               state,
                               is_shell,
                               is_companion,
                               header_title,
                               persona_name,
                               agent_type,
                               is_running, }| {
                       let menu_name = name.clone();
                       let tooltip_name = name.clone();
                       let menu_folder = folder.clone();
                       let folder_name =
                           PathBuf::from(&folder).file_name()
                                                 .map(|name| name.to_string_lossy().into_owned())
                                                 .unwrap_or(folder);
                       // Legacy/imported data may carry more
                       // than one character;
                       // clamp to a single grapheme so it can't
                       // overflow the tile.
                       let avatar = avatar.graphemes(true)
                                          .next()
                                          .unwrap_or(consts::DEFAULT_AGENT_AVATAR)
                                          .to_string();
                       let selected = self.selected_agent == Some(id);
                       // A plain clickable div, not `Button` -
                       // `Button`'s default
                       // sizing forces a fixed height
                       // regardless of content,
                       // clipping this row's up to 4 lines
                       // (name/persona/status/
                       // folder). Same fix as the dashboard
                       // cards in
                       // `dashboard.rs`.
                       div().id(gpui_kit::ElementId::from(format!("workspace-agent-{id}")))
                            .cursor_pointer()
                            .rounded(cx.theme().radius)
                            .p_2()
                            // A companion belongs to the agent above it, so it reads
                            // as nested: indented, with a rule down its left edge.
                            .when(is_companion, |row| {
                                row.ml_4().border_l_2().border_color(cx.theme().border)
                            })
                            .bg(if selected {
                                cx.theme().muted
                            }
                            else {
                                cx.theme().transparent
                            })
                            // A stopped agent's row is dimmed as a whole, so a
                            // workspace of mixed agents reads at a glance. The
                            // state dot cannot carry this: its four values say what
                            // a *running* agent is doing, and none of them means
                            // "not running at all". Selection still highlights the
                            // row underneath, so the selected-but-stopped agent
                            // whose pane shows the stopped placeholder is still
                            // visibly the selected one.
                            .when(!is_running, |row| row.opacity(0.45))
                            // The compact row hides the name, so the name becomes the
                            // row's tooltip - otherwise an avatar is the only thing
                            // left to tell two agents apart.
                            .when(compact, |row| {
                                row.tooltip(move |window, cx| {
                                       Tooltip::new(tooltip_name.clone()).build(window, cx)
                                   })
                            })
                            .child(if compact {
                                compact_agent_row_body(CompactAgentRow { avatar,
                                                                 state,
                                                                 is_shell }).into_any_element()
                            }
                            else {
                                h_flex()
                            .w_full()
                            .gap_3()
                            .items_start()
                            .child(
                                div()
                                    .w(px(40.))
                                    .h(px(40.))
                                    .flex_shrink_0()
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_2xl()
                                    .child(avatar),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .font_semibold()
                                            // A name is stored text, and
                                            // imported rosters carry
                                            // whatever the other tool held;
                                            // the flags above stop soft
                                            // wrapping only, so a name with
                                            // a line break in it would grow
                                            // the row.
                                            .child(single_line(&name)),
                                    )
                                    // A companion says so where a primary
                                    // agent names its type - it has no
                                    // coding-agent type of its own, and an
                                    // unlabelled row gave no clue what it was.
                                    .children(is_companion.then(|| {
                                        detail_line(
                                            gpui_kit::assets::IconName::CornerDownRight,
                                            knot_core::l10n::t("agent.companion"),
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                    }))
                                    // The agent type reads as one of the
                                    // row's detail lines, directly under the
                                    // name and above the persona - not
                                    // right-aligned opposite it, where it
                                    // floated away from the name it
                                    // describes and crowded the state dot.
                                    .children((!is_shell).then(|| {
                                        detail_line(
                                            SettingsWindow::agent_type_icon(&agent_type),
                                            SettingsWindow::agent_type_label(&agent_type)
                                                .to_string(),
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                    }))
                                    .children(persona_name.map(|persona_name| {
                                        // A drawn icon, not the `👤` this line
                                        // used to carry inside its text: an
                                        // emoji keeps its own colour and size,
                                        // so it was the one thing in the
                                        // column that did not line up.
                                        detail_line(
                                            gpui_kit::assets::IconName::User,
                                            persona_name,
                                            DetailLineSize::Small,
                                            title_font_name.clone(),
                                            title_font_size,
                                            cx,
                                        )
                                    }))
                                    .child(detail_line(
                                        gpui_kit::assets::IconName::Activity,
                                        header_title,
                                        DetailLineSize::Body,
                                        title_font_name.clone(),
                                        title_font_size,
                                        cx,
                                    ))
                                    .child(detail_line(
                                        gpui_kit::assets::IconName::Folder,
                                        folder_name,
                                        DetailLineSize::Body,
                                        title_font_name.clone(),
                                        title_font_size,
                                        cx,
                                    )),
                            )
                            // The state dot alone. The working indicator is
                            // deliberately not here: beside the dot it says
                            // the same thing twice, and the row already
                            // carries the one thing the dot cannot - a
                            // stopped agent's row is dimmed as a whole
                            // (see `agent-list-ui`).
                            .children((!is_shell).then(|| {
                                div()
                                    .flex_shrink_0()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .mt_1()
                                    .rounded_full()
                                    .bg(state_color(state))
                            }))
                            .into_any_element()
                            })
                            .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                                            view.select_agent(id);
                                            // Leave the dashboard, the same way
                                            // tapping an
                                            // agent card does - selecting a row
                                            // while the
                                            // dashboard was open used to change
                                            // the selection
                                            // without ever showing the session.
                                            view.view_mode = WorkspaceViewMode::Terminal;
                                            view.ensure_session(id);
                                            view.ensure_panel_session(id);
                                            cx.notify();
                                        }))
                            .context_menu({
                                let targets =
                                    AgentMenuTargets { store: Arc::clone(&store_for_menu),
                                                       settings: settings_for_menu.clone(),
                                                       window_entity: window_entity.clone(),
                                                       workspace_id,
                                                       id,
                                                       name: menu_name.clone(),
                                                       folder: menu_folder.clone() };
                                move |menu, window, cx| {
                                    agent_row_context_menu(&targets, menu, window, cx)
                                }
                            })
                   })
              .map(gpui_kit::IntoElement::into_any_element)
              .collect()
    }

    /// The workspace overview row, pinned above the agent list.
    ///
    /// `compact` drops its label, leaving the icon - the same trade the agent
    /// rows above it make at the same width.
    pub(super) fn dashboard_row(&self, is_dashboard: bool, compact: bool, cx: &mut Context<Self>)
                                -> gpui_kit::AnyElement {
        // The dashboard sits at the top of the agent list, the way the
        // Swift reference's `overviewRow` does (`SidebarView.swift`), and
        // not as an icon in the bottom bar: it is the workspace's overview
        // of every agent, so it belongs above them, shaped like the rows it
        // summarises. As a bare icon beside "New agent" it read as a minor
        // control and went unnoticed.
        div().id("workspace-dashboard-row")
             .cursor_pointer()
             .rounded(cx.theme().radius)
             .p_2()
             .bg(if is_dashboard {
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
                                        .child(Icon::default().path("icons/layout-dashboard.svg")))
                            .when(!compact, |row| {
                                row.child(div().flex_1()
                                               .min_w_0()
                                               .font_semibold()
                                               .child(knot_core::l10n::t("dashboard.title")))
                            }))
             .on_click(cx.listener(|view, _: &ClickEvent, _window, cx| {
                             view.view_mode = match view.view_mode {
                                 WorkspaceViewMode::Dashboard => WorkspaceViewMode::Terminal,
                                 _ => WorkspaceViewMode::Dashboard,
                             };
                             cx.notify();
                         }))
             .into_any_element()
    }
}
