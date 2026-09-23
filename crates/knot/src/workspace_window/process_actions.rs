//! What a process row can do: end it, carry its identity out of Knot, or
//! open the platform's process viewer.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "A listed process can
//! be terminated" and "A listed process can be identified outside Knot".
//!
//! Terminating asks first and then runs the whole `TERM` / grace / `KILL`
//! sequence on a blocking task. The row is marked terminating until a later
//! sample either drops it or shows it still running - nothing is removed
//! optimistically, because a signal that was refused must leave the list as
//! it was.

use std::sync::Arc;

use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::notification::Notification;
use gpui_kit::{ClipboardItem, Context, Window};
use knot_processes::{DescendantProcess, TerminationTarget};
use uuid::Uuid;

use super::window::WorkspaceWindow;
use crate::agent_processes::ProcessSection;

#[cfg(test)]
mod tests;

/// The identity the termination sequence re-verifies against, built from
/// the row the user clicked.
///
/// `None` when there is nothing to terminate: the agent has stopped, or the
/// row is older than the process it named. Neither is an error - the spec
/// calls an already-exited process a success, and the next sample drops the
/// row on its own.
pub(super) fn termination_target_for(root: Option<u32>, processes: Option<&[DescendantProcess]>,
                                     pid: u32)
                                     -> Option<TerminationTarget> {
    let root = root?;
    let process = processes?.iter().find(|process| process.pid == pid)?;

    Some(TerminationTarget { root,
                             pid: process.pid,
                             ppid: process.ppid,
                             command: process.command.clone(),
                             elapsed: process.elapsed })
}

/// Records a refused termination against the section that owns the row.
///
/// The list is left exactly as it was: the process is still running, and no
/// sample will ever remove it, so the terminating mark has to come off here
/// rather than waiting for one.
pub(super) fn record_termination_failure(section: &mut ProcessSection, pid: u32, reason: &str) {
    section.clear_terminating(pid);
    section.fail(knot_core::l10n::t_with("processes.terminate_failed", &[("reason", reason)]));
}

impl WorkspaceWindow {
    /// Asks before terminating, naming the command, and does nothing if the
    /// user cancels.
    pub(super) fn confirm_terminate_process(&mut self, agent_id: Uuid, pid: u32,
                                            window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.termination_target(agent_id, pid)
        else {
            // The row is older than the process: the next sample drops it,
            // and there is nothing to ask about.
            return;
        };

        let body = knot_core::l10n::t_with("processes.terminate_body",
                                           &[("command",
                                              &crate::app_support::single_line(&target.command))]);
        let entity = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let entity = entity.clone();
                  let target = target.clone();
                  alert.title(knot_core::l10n::t("processes.terminate_title"))
                       .description(body.clone())
                       // Named and tinted rather than a bare "OK": the
                       // action is destructive and cannot be undone, so the
                       // button says what it does.
                       .button_props(DialogButtonProps::default()
                           .ok_text(knot_core::l10n::t("processes.terminate"))
                           .ok_variant(ButtonVariant::Danger)
                           .show_cancel(true))
                       .on_ok(move |_, _, app| {
                           entity.update(app, |view, cx| {
                                     view.terminate_process(agent_id, target.clone());
                                     cx.notify();
                                 });
                           true
                       })
              });
    }

    /// Runs the termination sequence off the main thread.
    ///
    /// The row stays marked terminating until a sample says otherwise, so a
    /// process that ignores `TERM` for the whole grace period reads as being
    /// worked on rather than as a click that did nothing.
    fn terminate_process(&mut self, agent_id: Uuid, target: TerminationTarget) {
        let pid = target.pid;
        if let Some(section) = self.process_sections.get_mut(&agent_id) {
            section.mark_terminating(pid);
        }

        let failures = Arc::clone(&self.process_failures);
        self.runtime.spawn_blocking(move || {
                        if let Err(error) = knot_processes::terminate(&target) {
                            // A refused signal is the user's business: the
                            // process is not theirs to end, and the row stays
                            // exactly where it was.
                            failures.lock().push((agent_id, pid, error.to_string()));
                        }
                    });
    }

    /// Moves any termination failure into the section that owns the row.
    ///
    /// Answers whether one arrived, so the poll repaints for it.
    pub(super) fn drain_process_failures(&mut self) -> bool {
        let failures = std::mem::take(&mut *self.process_failures.lock());
        if failures.is_empty() {
            return false;
        }

        for (agent_id, pid, reason) in failures {
            if let Some(section) = self.process_sections.get_mut(&agent_id) {
                record_termination_failure(section, pid, &reason);
            }
        }

        true
    }

    fn termination_target(&self, agent_id: Uuid, pid: u32) -> Option<TerminationTarget> {
        let root = self.agent_session_root(agent_id);
        let processes = self.process_sections
                            .get(&agent_id)
                            .and_then(ProcessSection::processes);

        termination_target_for(root, processes, pid)
    }

    /// Copies the row's process identifier.
    ///
    /// Confirmed with a notification, as the code-block copy button is: a
    /// copy that says nothing is indistinguishable from a click that missed.
    pub(super) fn copy_process_pid(&self, pid: u32, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(pid.to_string()));
        window.push_notification(Notification::info(knot_core::l10n::t("processes.copied_pid")),
                                 cx);
    }

    /// Copies the row's command line in full.
    ///
    /// The command as the operating system reports it, not the text the row
    /// drew: that one is truncated to the pane's width and flattened onto one
    /// line, neither of which belongs on the clipboard.
    pub(super) fn copy_process_command(&self, command: String, window: &mut Window,
                                       cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(command));
        window.push_notification(
            Notification::info(knot_core::l10n::t("processes.copied_command")),
            cx,
        );
    }
}
