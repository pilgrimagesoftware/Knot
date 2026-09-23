//! The processes section: what the selected agent has left running.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "The agent's pane
//! presents a processes section".
//!
//! Mounted below whichever session pane the agent is showing, so the terminal
//! and panel views get it from one place rather than two that can drift. It
//! reads only what the sampler last published; nothing here enumerates
//! processes, which `agent_processes`'s own test enforces over these sources.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use knot_core::ViewMode;
use knot_processes::{Activity, DescendantProcess};
use uuid::Uuid;

use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;

#[cfg(test)]
mod tests;

/// Width of the runtime column, wide enough for `04d 05h` at the section's
/// text size.
const RUNTIME_WIDTH: f32 = 64.;
/// Width of the PID column, wide enough for a seven-digit identifier.
const PID_WIDTH: f32 = 64.;
/// How tall the expanded list grows before it scrolls, so a busy agent
/// cannot push the session pane off the window.
const BODY_MAX_HEIGHT: f32 = 220.;

/// Why an expanded section has no rows to show.
///
/// The spec forbids rendering blank: each case reads differently to the user,
/// and "nothing here" without saying which is the one that looks broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EmptyState {
    /// No session root: never started, deactivated, mid-restart.
    NotRunning,
    /// Running, and its tree is empty.
    NothingSpawned,
    /// No sample has completed yet.
    Counting,
}

impl EmptyState {
    pub(super) fn text(self) -> String {
        match self {
            Self::NotRunning => knot_core::l10n::t("processes.empty_not_running"),
            Self::NothingSpawned => knot_core::l10n::t("processes.empty_none"),
            Self::Counting => knot_core::l10n::t("processes.count_unknown"),
        }
    }
}

/// Which empty state an expanded section is in, or `None` when it has rows.
pub(super) fn empty_state(is_running: bool, processes: Option<&[DescendantProcess]>)
                          -> Option<EmptyState> {
    if !is_running {
        return Some(EmptyState::NotRunning);
    }

    match processes {
        None => Some(EmptyState::Counting),
        Some([]) => Some(EmptyState::NothingSpawned),
        Some(_) => None,
    }
}

pub(super) fn activity_text(activity: Activity) -> String {
    knot_core::l10n::t(match activity {
                           Activity::Background => "processes.background",
                           Activity::Foreground => "processes.foreground",
                       })
}

/// Elapsed runtime, at the coarsest two units that say something.
///
/// One catalog entry per shape rather than units glued together here: where
/// the number goes relative to its unit is the translator's to decide.
pub(super) fn runtime_text(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    let (days, hours, minutes, seconds) =
        (total / 86_400, (total % 86_400) / 3600, (total % 3600) / 60, total % 60);

    if days > 0 {
        return knot_core::l10n::t_with("processes.runtime_days",
                                       &[("days", &days.to_string()),
                                         ("hours", &hours.to_string())]);
    }
    if hours > 0 {
        return knot_core::l10n::t_with("processes.runtime_hours",
                                       &[("hours", &hours.to_string()),
                                         ("minutes", &minutes.to_string())]);
    }
    if minutes > 0 {
        return knot_core::l10n::t_with("processes.runtime_minutes",
                                       &[("minutes", &minutes.to_string()),
                                         ("seconds", &seconds.to_string())]);
    }

    knot_core::l10n::t_with("processes.runtime_seconds",
                            &[("seconds", &seconds.to_string())])
}

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

/// The four things a row shows, resolved before any element is built.
///
/// `command` is the process's own, verbatim and untruncated - the row
/// truncates visually, and the copy action must hand over the whole thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RowFields {
    pub(super) command:  String,
    pub(super) runtime:  String,
    pub(super) pid:      String,
    pub(super) activity: String,
}

/// One trailing control: an icon with a tooltip, ghost, as the UI
/// conventions ask for an action with an obvious icon.
///
/// `disabled` drops the click handler and the tooltip rather than dimming a
/// live control - a terminate already in flight has nothing more to ask for.
fn row_action(id: (&'static str, u64), icon: IconName, tooltip_key: &'static str,
              tint: gpui_kit::Hsla, disabled: bool,
              on_click: impl Fn(&ClickEvent, &mut Window, &mut gpui_kit::App) + 'static)
              -> gpui_kit::AnyElement {
    let tooltip = knot_core::l10n::t(tooltip_key);

    div().id(id)
         .p_1()
         .rounded_sm()
         .when(!disabled, |element| {
             element.cursor_pointer()
                    .on_click(on_click)
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
         })
         .child(Icon::new(icon).size_3().text_color(tint))
         .into_any_element()
}

pub(super) fn row_fields(process: &DescendantProcess) -> RowFields {
    RowFields { command:  single_line(&process.command),
                runtime:  runtime_text(process.elapsed),
                pid:      process.pid.to_string(),
                activity: activity_text(process.activity), }
}

impl WorkspaceWindow {
    /// The processes section for the shown agent, or `None` when no agent's
    /// session pane is on screen.
    ///
    /// Absent over the markdown and mermaid panes too: those are a document
    /// the agent put in front of the user, not its session.
    pub(super) fn processes_section(&mut self, layout: super::agent_sections::SectionLayout,
                                    cx: &mut Context<Self>)
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
        let summary = super::processes_summary::summary_text(
            is_running,
            expanded,
            self.process_section(agent_id)
                .and_then(crate::agent_processes::ProcessSection::processes),
        );

        // The divider between the two sections: a left border while they
        // share a row, a top border while they stack. Drawn here rather than
        // by the container so it is absent when this section is the only one
        // showing, which is every Terminal-mode agent.
        let alone = !super::mcp_pane::section_is_shown(view_mode,
                                                       self.view_mode.is_takeover(),
                                                       self.agent_has_document_pane(agent_id));
        let stacked = layout.is_stacked();

        Some(v_flex().map(|section| {
                         if stacked {
                             section.w_full()
                         }
                         else {
                             // `flex_1` alone keeps the default `min-width:
                             // auto`, so a collapsed header naming several
                             // processes would push its neighbour off the row
                             // instead of ellipsizing.
                             section.flex_1().min_w_0()
                         }
                     })
                     .when(!alone, |section| {
                         let section = if stacked {
                             section.border_t_1()
                         }
                         else {
                             section.border_l_1()
                         };

                         section.border_color(cx.theme().border)
                     })
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

    /// One process: its command on a single truncating line, then the
    /// columns and the actions, which never shrink.
    fn process_row(&self, agent_id: Uuid, process: &DescendantProcess, is_terminating: bool,
                   cx: &mut Context<Self>)
                   -> gpui_kit::AnyElement {
        let fields = row_fields(process);
        let pid = process.pid;
        let full_command = process.command.clone();

        h_flex().w_full()
                .min_w_0()
                .items_center()
                .gap_3()
                .px_5()
                .py_1()
                .hover(|style| style.bg(cx.theme().muted))
                // `min_w_0` on the growing child, not just `flex_1`: without it
                // the default `min-width: auto` lets a long command stretch the
                // row and push the columns off the pane.
                .child(div().flex_1()
                            .min_w_0()
                            .text_sm()
                            .font_family(cx.theme().mono_font_family.clone())
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(fields.command.clone()))
                .child(div().flex_shrink_0()
                            .w(px(RUNTIME_WIDTH))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(if is_terminating {
                                       knot_core::l10n::t("processes.terminating")
                                   }
                                   else {
                                       fields.runtime.clone()
                                   }))
                .child(div().flex_shrink_0()
                            .w(px(PID_WIDTH))
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(fields.pid.clone()))
                .child(div().flex_shrink_0()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(fields.activity.clone()))
                .child(self.process_row_actions(agent_id, pid, full_command, is_terminating, cx))
                .into_any_element()
    }

    /// The row's trailing controls: terminate, the two copies, and the
    /// process viewer where the platform has one.
    ///
    /// Icon buttons with tooltips rather than labels, per
    /// `.claude/rules/knot-ui-conventions.md`; the terminate icon is tinted
    /// red while the button stays ghost, since `danger` would replace the
    /// ghost variant outright.
    fn process_row_actions(&self, agent_id: Uuid, pid: u32, command: String,
                           is_terminating: bool, cx: &mut Context<Self>)
                           -> gpui_kit::AnyElement {
        let copy_command = command.clone();
        let danger = cx.theme().danger;
        let muted = cx.theme().muted_foreground;

        h_flex().flex_shrink_0()
                .items_center()
                .gap_1()
                .child(row_action(("terminate-process", u64::from(pid)),
                                  IconName::Close,
                                  "processes.terminate",
                                  danger,
                                  is_terminating,
                                  cx.listener(move |view, _: &ClickEvent, window, cx| {
                                        view.confirm_terminate_process(agent_id, pid, window, cx);
                                    })))
                .child(row_action(("copy-process-pid", u64::from(pid)),
                                  IconName::Hash,
                                  "processes.copy_pid",
                                  muted,
                                  false,
                                  cx.listener(move |view, _: &ClickEvent, window, cx| {
                                        view.copy_process_pid(pid, window, cx);
                                    })))
                .child(row_action(("copy-process-command", u64::from(pid)),
                                  IconName::Copy,
                                  "processes.copy_command",
                                  muted,
                                  false,
                                  cx.listener(move |view, _: &ClickEvent, window, cx| {
                                        view.copy_process_command(copy_command.clone(), window, cx);
                                    })))
                // Left out entirely where the platform has no process viewer,
                // rather than drawn as a control that does nothing.
                .children(crate::open_in::has_process_viewer().then(|| {
                              row_action(("open-process-viewer", u64::from(pid)),
                                     IconName::ExternalLink,
                                     "processes.open_viewer",
                                     muted,
                                     false,
                                     |_: &ClickEvent, _window: &mut Window,
                                      _app: &mut gpui_kit::App| {
                                         crate::open_in::open_process_viewer();
                                     })
                          }))
                .into_any_element()
    }

    /// Whether a document the agent opened has taken the content area, in
    /// which case its session pane - and this section with it - is not shown.
    pub(super) fn agent_has_document_pane(&self, agent_id: Uuid) -> bool {
        let store = self.store.lock();
        let Some(agent) = store.agent(agent_id)
        else {
            return false;
        };

        agent.markdown_file.is_some() || agent.mermaid_source.is_some()
    }
}
