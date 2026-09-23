//! Running a `!` command from the panel composer, and getting its result to
//! the agent.
//!
//! Contract: `openspec/specs/panel-shell-passthrough/spec.md`.
//!
//! The command runs locally, in the agent's folder, and never touches the ACP
//! session: it does not start a turn, does not join the prompt queue, and is
//! not held by a pending permission request. The only path from a command to
//! the agent is [`WorkspaceWindow::take_panel_shell_context`], which the next
//! ordinary prompt calls.
//!
//! Live runs are held here, in `panel_shell_runs`, rather than in the card
//! that draws them: a `PanelMessage` is cloned and compared once per frame,
//! and a lock in one would put the render path behind the command's drain
//! threads. [`WorkspaceWindow::poll_panel_shell_runs`] copies from the handles
//! into the cards, and is in `repaint_poll_tick`'s chain - without that, a
//! command's output would reach `PanelState` and stop there.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::Context;
use knot_processes::{ShellRequest, ShellRun};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::panel_session;
use crate::panel_state::ShellCard;
use crate::workspace_window::WorkspaceWindow;

#[cfg(test)]
mod tests;

/// One live command: which agent's conversation holds its card, and the
/// handle to the process itself.
pub(in crate::workspace_window) struct PanelShellRun {
    /// The agent whose panel the card lives in. Kept so a finished run can
    /// find its conversation without searching every panel.
    pub(in crate::workspace_window) agent: Uuid,
    pub(in crate::workspace_window) run:   ShellRun,
}

/// The window's live-run table, shared so a render closure can cancel one
/// without a `Context` to reach the window through.
pub(in crate::workspace_window) type PanelShellRuns = Arc<Mutex<BTreeMap<Uuid, PanelShellRun>>>;

/// Stops the command `card_id` draws, if it is still running.
///
/// Free function rather than a method: the cancel control is built during a
/// render and captures a clone of the table, never the window.
pub(in crate::workspace_window) fn cancel_run(runs: &PanelShellRuns, card_id: Uuid) {
    if let Some(shell) = runs.lock().get(&card_id) {
        shell.run.cancel();
    }
}

impl WorkspaceWindow {
    /// Runs `command` in `id`'s folder and puts its card in the
    /// conversation.
    ///
    /// Returns whether the command was started. A panel with no live session
    /// has no conversation to put the card in, so nothing runs and the
    /// composer keeps what the user typed - the same answer
    /// `deliver_panel_prompt` gives for a prompt.
    pub(in crate::workspace_window) fn run_panel_shell_command(&mut self, id: Uuid,
                                                               command: String,
                                                               cx: &mut Context<Self>)
                                                               -> bool {
        // The same lookup the git panel and the skill registry already do, so
        // a command runs exactly where the agent does.
        let Some(folder) = self.agent_folder(id)
        else {
            return false;
        };
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };

        let recorder = {
            let guard = slot.lock();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => Some(handle.recorder()),
                _ => None,
            }
        };
        let Some(recorder) = recorder
        else {
            return false;
        };

        let card_id = Uuid::new_v4();
        let request = ShellRequest::new(command.clone(), &folder);
        let run = knot_processes::shell::spawn(request);

        recorder.shell_command(ShellCard::starting(card_id, command, folder, run.snapshot()));
        self.panel_shell_runs
            .lock()
            .insert(card_id, PanelShellRun { agent: id, run });
        cx.notify();
        true
    }

    /// Copies every changed run into its card, reporting whether a frame
    /// would draw differently.
    ///
    /// Belongs in `repaint_poll_tick`'s `if` chain. The `take_dirty()` read
    /// clears as it reads, so it is taken into a local *before* anything that
    /// could skip it: a `||` chain would short-circuit past a run whose
    /// output had arrived, and the flag would stay clear until the next
    /// append.
    pub(in crate::workspace_window) fn poll_panel_shell_runs(&mut self) -> bool {
        // Collected under the table's lock and applied after it is dropped:
        // this is the one path that would otherwise hold the run table while
        // taking a session slot, and every other path takes them the other
        // way round.
        let landed: Vec<(Uuid, Uuid, knot_processes::ShellRunState)> = {
            let runs = self.panel_shell_runs.lock();
            runs.iter()
                .filter(|(_, shell)| shell.run.take_dirty())
                .map(|(card_id, shell)| (*card_id, shell.agent, shell.run.snapshot()))
                .collect()
        };

        if landed.is_empty() {
            return false;
        }

        let mut finished: Vec<Uuid> = Vec::new();

        for (card_id, agent, snapshot) in landed {
            if snapshot.status.is_terminal() {
                finished.push(card_id);
            }

            if let Some(state) = self.panel_state_of(agent) {
                state.lock().update_shell_command(card_id, snapshot);
            }
        }

        // A finished run has nothing further to report, and its card holds
        // everything the conversation needs. Keeping the handle would keep
        // the output buffers alive twice over, once per command, forever.
        if !finished.is_empty() {
            let mut runs = self.panel_shell_runs.lock();
            for card_id in finished {
                runs.remove(&card_id);
            }
        }

        true
    }

    /// Everything `id`'s pending commands have to tell the agent, as text to
    /// append to the prompt carrying them, and marks them shared.
    ///
    /// Empty when nothing is pending, which is the ordinary case - a prompt
    /// with no `!` command before it is unchanged by this.
    pub(in crate::workspace_window) fn take_panel_shell_context(&mut self, id: Uuid) -> String {
        let Some(state) = self.panel_state_of(id)
        else {
            return String::new();
        };

        let mut state = state.lock();
        let context: String = state.pending_shell_results()
                                   .iter()
                                   .map(|card| shell_context_block(card))
                                   .collect();

        if !context.is_empty() {
            state.mark_shell_results_shared();
        }

        context
    }

    fn panel_state_of(
        &self, id: Uuid)
        -> Option<std::sync::Arc<parking_lot::Mutex<crate::panel_state::PanelState>>> {
        let slot = self.panel_sessions.get(&id)?;
        let guard = slot.lock();
        match &*guard {
            panel_session::PanelSessionSlot::Ready(handle) => Some(handle.state()),
            _ => None,
        }
    }
}

/// One command's contribution to the prompt that carries it.
///
/// The same shape as the attached-context lines already appended by
/// `send_panel_prompt`: `knot-acp`'s `session/prompt` sends a single text
/// block, so this is text, not an invented resource-attachment wire shape.
/// It is labelled as the user's own command so the agent does not read the
/// output as something it produced.
fn shell_context_block(card: &crate::panel_state::ShellCard) -> String {
    let mut block = format!("\n\nI ran this in {}:\n\n$ {}\n", card.cwd, card.command);

    if !card.run.stdout.is_empty() {
        block.push_str(&format!("\n{}\n", card.run.stdout.text().trim_end()));
    }

    if !card.run.stderr.is_empty() {
        block.push_str(&format!("\nstderr:\n{}\n", card.run.stderr.text().trim_end()));
    }

    if card.run.is_truncated() {
        block.push_str("\n(output was truncated)\n");
    }

    block.push_str(&format!("\n{}\n", exit_line(card)));
    block
}

fn exit_line(card: &crate::panel_state::ShellCard) -> String {
    use knot_processes::ShellStatus;

    match card.status() {
        ShellStatus::Exited { code: 0 } => "(exit 0)".to_owned(),
        ShellStatus::Exited { code } => format!("(exit {code})"),
        // The only other status that can be pending; see
        // `ShellStatus::ran_to_completion`.
        _ => "(ended by a signal)".to_owned(),
    }
}
