//! One operating-system process, as a row.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - the row's fields, and
//! "A listed process can be terminated" / "identified outside Knot" for its
//! trailing controls.

use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, px,
};
use knot_processes::DescendantProcess;
use uuid::Uuid;

use super::chrome::{PID_WIDTH, RUNTIME_WIDTH, row_action};
use super::text::{activity_text, runtime_text};
use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;

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

pub(super) fn row_fields(process: &DescendantProcess) -> RowFields {
    RowFields { command:  single_line(&process.command),
                runtime:  runtime_text(process.elapsed),
                pid:      process.pid.to_string(),
                activity: activity_text(process.activity), }
}

impl WorkspaceWindow {
    /// One process: its command on a single truncating line, then the
    /// columns and the actions, which never shrink.
    pub(super) fn process_row(&self, agent_id: Uuid, process: &DescendantProcess,
                              is_terminating: bool, cx: &mut Context<Self>)
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
}
