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
use knot_subagents::Subagent;
use uuid::Uuid;

use super::chrome::{BODY_MAX_HEIGHT, group_label};
use super::empty::{EmptyState, empty_state, subagent_empty_state};
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

    /// The expanded body: two labelled groups, subagents before processes.
    ///
    /// Adjacent, never nested. A process a subagent runs descends from the
    /// same session root as everything else the agent runs, so no attribution
    /// can be established from `ps` - and indenting the second group under
    /// the first would claim one.
    fn processes_body(&mut self, agent_id: Uuid, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let is_running = self.agent_session_root(agent_id).is_some();

        div().id("processes-body")
             .w_full()
             .max_h(px(BODY_MAX_HEIGHT))
             .overflow_y_scroll()
             .child(v_flex().w_full()
                            .children(self.subagents_group(agent_id, is_running, cx))
                            .child(self.processes_group(agent_id, is_running, cx)))
             .into_any_element()
    }

    /// The subagents group, or `None` for an agent whose type cannot report
    /// them.
    ///
    /// Omitted entirely rather than shown empty: "has dispatched none" and
    /// "Knot cannot tell" are opposite answers, and a group that said the
    /// first when the second is true would be the section's one real lie.
    fn subagents_group(&mut self, agent_id: Uuid, is_running: bool, cx: &mut Context<Self>)
                       -> Option<gpui_kit::AnyElement> {
        if !self.agent_reports_subagents(agent_id) {
            return None;
        }

        // One lock, one clock, one copy - the same bargain the process rows
        // make with the sampler's snapshot. Holding the guard across the row
        // building would put a lock the two feeds also write to on the render
        // path for the length of a frame.
        let now = std::time::Instant::now();
        let subagents: Vec<Subagent> = {
            let registry = self.subagents.lock();
            registry.ordered(agent_id, now)
                    .into_iter()
                    .cloned()
                    .collect()
        };
        let empty = subagent_empty_state(is_running, subagents.len());

        Some(v_flex().w_full()
                     .child(group_label("processes.group_subagents", cx.theme().muted_foreground))
                     .children(empty.map(|state| empty_line(state, cx)))
                     .children(subagents.iter()
                                        .map(|subagent| self.subagent_row(subagent, now, cx)))
                     .into_any_element())
    }

    /// The processes group: the rows, or the one line saying why there are
    /// none, with any sample failure reported alongside.
    fn processes_group(&mut self, agent_id: Uuid, is_running: bool, cx: &mut Context<Self>)
                       -> gpui_kit::AnyElement {
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

        v_flex().w_full()
                .child(group_label("processes.group_processes", cx.theme().muted_foreground))
                .children(failure.map(|reason| {
                                     div().px_5()
                                          .py_1()
                                          .text_xs()
                                          .text_color(cx.theme().danger)
                                          .child(knot_core::l10n::t_with("processes.sample_failed",
                                                                         &[("reason", &reason)]))
                                 }))
                .children(empty.map(|state| empty_line(state, cx)))
                .children(processes.into_iter().flatten().map(|process| {
                                                             let is_terminating =
                                                                 terminating.contains(&process.pid);
                                                             self.process_row(agent_id,
                                                                              &process,
                                                                              is_terminating,
                                                                              cx)
                                                         }))
                .into_any_element()
    }

    /// Whether this agent's type can report subagents in the view mode it is
    /// running in.
    fn agent_reports_subagents(&self, agent_id: Uuid) -> bool {
        let store = self.store.lock();
        let Some(agent) = store.agent(agent_id)
        else {
            return false;
        };

        knot_core::agent_type::reports_subagents(&agent.agent_type, agent.view_mode)
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

/// One group's "why there is nothing here" line.
///
/// Shared by both groups so the two read identically - a user comparing them
/// is comparing the words, not the typography.
fn empty_line(state: EmptyState, cx: &mut Context<WorkspaceWindow>) -> gpui_kit::AnyElement {
    div().px_5()
         .py_2()
         .text_sm()
         .text_color(cx.theme().muted_foreground)
         .child(state.text())
         .into_any_element()
}
