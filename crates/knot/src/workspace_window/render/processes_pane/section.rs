//! The section itself: whether it is shown, its header, and its body.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "The agent's pane
//! presents a processes section".
//!
//! Mounted below whichever session pane the agent is showing, so the terminal
//! and panel views get it from one place rather than two that can drift.

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use knot_core::ViewMode;
use knot_processes::DescendantProcess;
use uuid::Uuid;

use super::chrome::BODY_MAX_HEIGHT;
use super::empty::empty_state;
use crate::workspace_window::WorkspaceWindow;

/// Whether the section belongs under the content area right now.
///
/// `view_mode` is taken and deliberately not matched on: the section is
/// mounted below whichever session pane is showing, so a Terminal agent and
/// a Panel agent get the same answer under the same conditions. That is the
/// spec's "the section appears in both views", and passing the mode in is
/// what lets a test hold it.
pub(super) fn section_is_shown(view_mode: ViewMode, is_takeover: bool, has_document_pane: bool)
                               -> bool {
    let _ = view_mode;

    !is_takeover && !has_document_pane
}

impl WorkspaceWindow {
    /// The processes section for the shown agent, or `None` when no agent's
    /// session pane is on screen.
    ///
    /// Absent over the markdown and mermaid panes too: those are a document
    /// the agent put in front of the user, not its session.
    pub(in crate::workspace_window::render) fn processes_section(
        &mut self, cx: &mut Context<Self>)
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

        let expanded = self.process_section(agent_id)
                           .is_some_and(crate::agent_processes::ProcessSection::is_expanded);
        let is_running = self.agent_session_root(agent_id).is_some();
        let summary = crate::workspace_window::render::processes_summary::summary_text(
            is_running,
            expanded,
            self.process_section(agent_id)
                .and_then(crate::agent_processes::ProcessSection::processes),
        );

        Some(v_flex().w_full()
                     .flex_shrink_0()
                     .border_t_1()
                     .border_color(cx.theme().border)
                     .bg(cx.theme().background)
                     .child(self.processes_header(agent_id, expanded, summary, cx))
                     .children(expanded.then(|| self.processes_body(agent_id, cx)))
                     .into_any_element())
    }

    /// The always-visible header: a disclosure triangle, the label, and the
    /// summary - names while collapsed, a count while expanded.
    fn processes_header(&self, agent_id: Uuid, expanded: bool, summary: String,
                        cx: &mut Context<Self>)
                        -> gpui_kit::AnyElement {
        h_flex().id("processes-header")
                .w_full()
                .items_center()
                .gap_2()
                .px_5()
                .py_2()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().muted))
                .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                                view.toggle_process_section(agent_id);
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
                            .child(knot_core::l10n::t("processes.title")))
                // `min_w_0` + truncation: a collapsed header naming several
                // processes must not widen the pane.
                .child(div().min_w_0()
                            .text_sm()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .text_color(cx.theme().muted_foreground)
                            .child(summary))
                .into_any_element()
    }

    /// The expanded body: the rows, or the one line saying why there are
    /// none, with any sample failure reported alongside.
    fn processes_body(&mut self, agent_id: Uuid, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let is_running = self.agent_session_root(agent_id).is_some();
        let section = self.process_section(agent_id);
        let processes: Option<Vec<DescendantProcess>> =
            section.and_then(|section| section.processes().map(<[_]>::to_vec));
        let failure = section.and_then(|section| section.failure().map(str::to_owned));
        let terminating: Vec<u32> = processes.iter()
                                             .flatten()
                                             .map(|process| process.pid)
                                             .filter(|pid| {
                                                 self.process_section(agent_id)
                                                     .is_some_and(|s| s.is_terminating(*pid))
                                             })
                                             .collect();
        let empty = empty_state(is_running, processes.as_deref());

        div()
            .id("processes-body")
            .w_full()
            .max_h(px(BODY_MAX_HEIGHT))
            .overflow_y_scroll()
            .child(v_flex().w_full().children(failure.map(|reason| {
                          div().px_5()
                               .py_1()
                               .text_xs()
                               .text_color(cx.theme().danger)
                               .child(knot_core::l10n::t_with("processes.sample_failed",
                                                              &[("reason", &reason)]))
                      }))
            .children(empty.map(|state| {
                          div().px_5()
                               .py_2()
                               .text_sm()
                               .text_color(cx.theme().muted_foreground)
                               .child(state.text())
                      }))
            .children(processes.into_iter().flatten().map(|process| {
                          let is_terminating = terminating.contains(&process.pid);
                          self.process_row(agent_id, &process, is_terminating, cx)
                      })))
            .into_any_element()
    }

    /// Whether a document the agent opened has taken the content area, in
    /// which case its session pane - and this section with it - is not shown.
    fn agent_has_document_pane(&self, agent_id: Uuid) -> bool {
        let store = self.store.lock();
        let Some(agent) = store.agent(agent_id)
        else {
            return false;
        };

        agent.markdown_file.is_some() || agent.mermaid_source.is_some()
    }
}
