//! The MCP servers section: which servers the shown agent can reach.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md`.
//!
//! Mounted below the session pane beside the processes section, and only for
//! a Panel-mode agent. A Terminal agent has a real PTY, so its own `/mcp`
//! already works there and a second way to ask would be two answers to one
//! question.
//!
//! Nothing here starts a probe. The section renders what
//! `mcp_panel::probe`'s tick last published, which a test in that module
//! enforces over this directory - a probe on the render path would run an
//! agent's CLI per keystroke.

use std::time::{Duration, Instant};

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use knot_core::ViewMode;
use knot_mcp_probe::ServerState;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::mcp_panel::rows::{SectionRow, compose};
use crate::workspace_window::mcp_panel::summary::{SectionStatus, state_label, summary_text};

#[cfg(test)]
mod tests;

/// Width of the state column, wide enough for the longest state label at the
/// section's text size.
const STATE_WIDTH: f32 = 148.;
/// How tall the expanded list grows before it scrolls, so an agent with many
/// servers cannot push the session pane off the window.
const BODY_MAX_HEIGHT: f32 = 220.;

/// Whether the section belongs under the content area right now.
///
/// Panel mode only, and the one place that decides it. A Terminal agent's own
/// `/mcp` works; a takeover or a document pane is not the agent's session.
pub(super) fn section_is_shown(view_mode: ViewMode, is_takeover: bool, has_document_pane: bool)
                               -> bool {
    view_mode == ViewMode::Panel && !is_takeover && !has_document_pane
}

/// How long ago a probe ran, in words.
///
/// The section says this rather than presenting the rows as live, because
/// they are a separate process's view of the same configuration taken at some
/// earlier moment. One catalog entry per shape: where the number sits
/// relative to its unit is the translator's to decide.
pub(super) fn taken_ago_text(elapsed: Duration) -> String {
    let total = elapsed.as_secs();

    if total < 5 {
        return knot_core::l10n::t("mcp.taken_just_now");
    }

    let age = if total < 60 {
        knot_core::l10n::t_with("mcp.age_seconds", &[("seconds", &total.to_string())])
    }
    else if total < 3600 {
        knot_core::l10n::t_with("mcp.age_minutes", &[("minutes", &(total / 60).to_string())])
    }
    else {
        knot_core::l10n::t_with("mcp.age_hours", &[("hours", &(total / 3600).to_string())])
    };

    knot_core::l10n::t_with("mcp.taken_at", &[("age", &age)])
}

/// The colour a state reads in.
///
/// Only the two that are asking for something are tinted: a disabled or
/// pending server is the user's own choice, and colouring it as a problem
/// would say the opposite of what the state means.
fn state_color(state: ServerState, cx: &Context<WorkspaceWindow>) -> gpui_kit::Hsla {
    match state {
        ServerState::Connected => cx.theme().success,
        ServerState::NeedsAuthentication => cx.theme().warning,
        ServerState::Failed => cx.theme().danger,
        ServerState::PendingApproval | ServerState::Disabled | ServerState::Unknown => {
            cx.theme().muted_foreground
        }
    }
}

impl WorkspaceWindow {
    /// The MCP section for the shown agent, or `None` when it does not
    /// belong on screen.
    pub(super) fn mcp_servers_section(&mut self, cx: &mut Context<Self>)
                                      -> Option<gpui_kit::AnyElement> {
        let agent_id = self.selected_agent?;
        let view_mode = self.store
                            .lock()
                            .agent(agent_id)
                            .map(|agent| agent.view_mode)?;

        if !section_is_shown(view_mode,
                             self.view_mode.is_takeover(),
                             self.agent_has_document_pane(agent_id))
        {
            return None;
        }

        let expanded = self.mcp_section(agent_id)
                           .is_some_and(|section| section.expanded);
        let rows = self.mcp_rows(agent_id, cx);
        let status = SectionStatus { is_running: self.agent_session_root(agent_id).is_some(),
                                     expanded,
                                     probing: self.mcp_probing(agent_id),
                                     inventory: self.mcp_section(agent_id)
                                                    .and_then(|section| section.inventory()),
                                     failure: self.mcp_section(agent_id)
                                                  .and_then(|section| section.failure()),
                                     row_count: rows.len() };
        let summary = summary_text(&status);

        Some(v_flex().w_full()
                     .flex_shrink_0()
                     .border_t_1()
                     .border_color(cx.theme().border)
                     .bg(cx.theme().background)
                     .child(self.mcp_header(agent_id, expanded, summary, cx))
                     .children(expanded.then(|| self.mcp_body(agent_id, rows, cx)))
                     .into_any_element())
    }

    /// The rows this agent's section shows: Knot's own, then the agent's own.
    ///
    /// Knot's state comes from the global its supervisor publishes to, not
    /// from any probe - it is authoritative and available before a probe has
    /// run. A build with no supervisor running (a test window) reads as
    /// disabled rather than panicking on the missing global.
    fn mcp_rows(&self, agent_id: Uuid, cx: &Context<Self>) -> Vec<SectionRow> {
        let port = self.settings.mcp_server_port;
        let knot_state = if cx.has_global::<crate::mcp_status::McpServerStatus>() {
            cx.global::<crate::mcp_status::McpServerStatus>().state()
        }
        else {
            knot_mcp::ServerState::Disabled
        };

        compose(knot_state,
                &crate::settings_window::SettingsWindow::mcp_server_url(port),
                self.mcp_section(agent_id)
                    .and_then(|section| section.inventory()))
    }

    /// The always-visible header: a disclosure triangle, the label, the
    /// summary, and the refresh action.
    fn mcp_header(&self, agent_id: Uuid, expanded: bool, summary: String, cx: &mut Context<Self>)
                  -> gpui_kit::AnyElement {
        let refresh_tooltip = knot_core::l10n::t("mcp.action_refresh");

        h_flex().id("mcp-header")
                .w_full()
                .items_center()
                .gap_2()
                .px_5()
                .py_2()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().muted))
                .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                                view.toggle_mcp_section(agent_id);
                                cx.notify();
                            }))
                .child(Icon::new(if expanded {
                                     IconName::ChevronDown
                                 }
                                 else {
                                     IconName::ChevronRight
                                 }).size_4()
                                   .text_color(cx.theme().muted_foreground))
                .child(div().text_sm()
                            .font_semibold()
                            .child(knot_core::l10n::t("mcp.title")))
                // `min_w_0` + truncation: a collapsed header naming several
                // servers must not widen the pane.
                .child(div().min_w_0()
                            .flex_1()
                            .text_sm()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(cx.theme().muted_foreground)
                            .child(summary))
                .child(div().id("mcp-refresh")
                            .p_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .tooltip(move |window, cx| {
                                Tooltip::new(refresh_tooltip.clone()).build(window, cx)
                            })
                            .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                                            view.request_mcp_probe(agent_id);
                                            cx.notify();
                                        }))
                            .child(Icon::new(IconName::RefreshCw).size_3()
                                                                 .text_color(cx.theme()
                                                                               .muted_foreground)))
                .into_any_element()
    }

    /// The expanded body: one line per server, and the age of the rows.
    fn mcp_body(&mut self, agent_id: Uuid, rows: Vec<SectionRow>, cx: &mut Context<Self>)
                -> gpui_kit::AnyElement {
        let taken = self.mcp_section(agent_id)
                        .and_then(|section| section.inventory())
                        .and_then(knot_mcp_probe::Inventory::taken_at)
                        .map(|at| taken_ago_text(Instant::now().saturating_duration_since(at)));

        v_flex().id("mcp-body")
                .w_full()
                .max_h(px(BODY_MAX_HEIGHT))
                .overflow_y_scroll()
                .children(rows.into_iter()
                              .enumerate()
                              .map(|(index, row)| self.mcp_row(index, &row, cx)))
                .children(taken.map(|text| {
                                   div().px_5()
                                        .py_1()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(text)
                               }))
                .into_any_element()
    }

    /// One server's row: its name, how it is reached, and what state it is
    /// in - with the full address behind a copy action.
    ///
    /// The address is drawn through `short_label`, never whole. A stdio
    /// server's command line is unbounded, and this list is virtualized: a
    /// row taller than it declared corrupts the scroll position of everything
    /// below it.
    fn mcp_row(&self, index: usize, row: &SectionRow, cx: &mut Context<Self>)
               -> gpui_kit::AnyElement {
        let (label, transport, state_text, color, detail, full) = match row {
            SectionRow::Knot(knot) => {
                (knot_acp::MCP_SERVER_NAME.to_owned(),
                 "http",
                 crate::settings_window::SettingsWindow::mcp_state_text(&knot.state),
                 cx.theme().muted_foreground,
                 knot.also_configured
                     .then(|| knot_core::l10n::t("mcp.also_configured")),
                 knot.url.clone())
            }
            SectionRow::Agent(server) => (server.name.clone(),
                                          server.target.token(),
                                          state_label(server.state),
                                          state_color(server.state, cx),
                                          server.detail.clone(),
                                          server.target.full().to_owned()),
        };

        let copy_tooltip = knot_core::l10n::t("mcp.action_copy_target");
        let address = row.short_label();

        v_flex()
            .w_full()
            .px_5()
            .py_1()
            .gap_0()
            .child(h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .child(div().min_w_0().text_sm().whitespace_nowrap().text_ellipsis().child(single_line(&label)))
                .child(div().text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(transport))
                .child(div().min_w_0()
                            .flex_1()
                            .text_xs()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(address))
                .child(div().id(("mcp-copy", index as u64))
                            .p_1()
                            .rounded_sm()
                            .cursor_pointer()
                            .tooltip(move |window, cx| {
                                Tooltip::new(copy_tooltip.clone()).build(window, cx)
                            })
                            .on_click(move |_: &ClickEvent, _window, app: &mut gpui_kit::App| {
                                app.write_to_clipboard(gpui_kit::ClipboardItem::new_string(full.clone()));
                            })
                            .child(Icon::new(IconName::Copy)
                                .size_3()
                                .text_color(cx.theme().muted_foreground)))
                .child(div().w(px(STATE_WIDTH))
                            .text_xs()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(color)
                            .child(state_text)))
            .children(detail.map(|text| {
                          div().text_xs()
                               .whitespace_nowrap()
                               .text_ellipsis()
                               .min_w_0()
                               .text_color(cx.theme().muted_foreground)
                               .child(single_line(&text))
                      }))
            .into_any_element()
    }
}

impl SectionRow {
    /// The bounded address a row draws.
    fn short_label(&self) -> String {
        match self {
            Self::Knot(knot) => knot.url.clone(),
            Self::Agent(row) => row.short_label(),
        }
    }
}
