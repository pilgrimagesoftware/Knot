//! The shell-command entry's own state: what a card starts as, when its
//! result becomes shareable, and what clears it again.
//!
//! The rule under most of these is that only a command which ran to its own
//! end may reach the agent. It is asserted per outcome rather than once,
//! because each outcome arrives by a different path at run time.

use knot_processes::{ShellRunState, ShellStatus, ShellStream};
use uuid::Uuid;

use crate::panel_state::{PanelMessage, PanelState, ShellCard, ShellDelivery};

fn run(status: ShellStatus) -> ShellRunState {
    let mut state = ShellRunState::new(1024);
    state.status = status;
    state
}

fn card(state: &mut PanelState, command: &str) -> Uuid {
    let id = Uuid::new_v4();
    state.push_shell_command(ShellCard::starting(id,
                                                 command.to_owned(),
                                                 "/tmp/worktree".to_owned(),
                                                 ShellRunState::new(1024)));
    id
}

#[test]
fn a_submitted_command_starts_running_and_unshareable() {
    let mut state = PanelState::default();
    let id = card(&mut state, "ls -la");

    let card = state.shell_card(id).expect("card was pushed");

    assert!(card.is_running());
    assert_eq!(card.delivery, ShellDelivery::None);
    assert_eq!(card.command, "ls -la");
    assert_eq!(card.cwd, "/tmp/worktree");
}

#[test]
fn a_shell_command_does_not_start_a_turn() {
    let mut state = PanelState::default();

    card(&mut state, "ls");

    assert!(!state.turn_active,
            "running a command is not the agent answering");
}

#[test]
fn a_command_that_exits_becomes_pending() {
    let mut state = PanelState::default();
    let id = card(&mut state, "ls");

    assert!(state.update_shell_command(id, run(ShellStatus::Exited { code: 0 })));

    assert!(state.shell_card(id).unwrap().is_pending());
}

#[test]
fn a_failing_command_is_still_shareable() {
    let mut state = PanelState::default();
    let id = card(&mut state, "cargo test");

    state.update_shell_command(id, run(ShellStatus::Exited { code: 101 }));

    assert!(state.shell_card(id).unwrap().is_pending(),
            "a failure is the answer to 'did it pass?'");
}

#[test]
fn a_command_that_did_not_end_on_its_own_is_never_pending() {
    for status in [ShellStatus::Cancelled,
                   ShellStatus::TimedOut,
                   ShellStatus::FailedToStart { message: "no shell".to_owned(), }]
    {
        let mut state = PanelState::default();
        let id = card(&mut state, "sleep 30");

        state.update_shell_command(id, run(status.clone()));

        assert_eq!(state.shell_card(id).unwrap().delivery,
                   ShellDelivery::None,
                   "{status:?} must not be shareable");
    }
}

#[test]
fn a_running_command_is_not_pending() {
    let mut state = PanelState::default();
    let id = card(&mut state, "sleep 30");

    let mut partial = ShellRunState::new(1024);
    partial.append(ShellStream::Stdout, "working");
    state.update_shell_command(id, partial);

    let card = state.shell_card(id).unwrap();

    assert!(card.is_running());
    assert!(!card.is_pending());
    assert_eq!(card.run.stdout.text(), "working");
}

#[test]
fn pending_results_come_back_in_submission_order() {
    let mut state = PanelState::default();
    let first = card(&mut state, "git status");
    let second = card(&mut state, "git diff");

    state.update_shell_command(second, run(ShellStatus::Exited { code: 0 }));
    state.update_shell_command(first, run(ShellStatus::Exited { code: 0 }));

    let commands: Vec<&str> = state.pending_shell_results()
                                   .iter()
                                   .map(|card| card.command.as_str())
                                   .collect();

    assert_eq!(commands,
               vec!["git status", "git diff"],
               "order is the order they were submitted, not the order they finished");
}

#[test]
fn sharing_marks_every_pending_result_and_leaves_the_cards() {
    let mut state = PanelState::default();
    let id = card(&mut state, "ls");
    state.update_shell_command(id, run(ShellStatus::Exited { code: 0 }));

    state.mark_shell_results_shared();

    assert_eq!(state.shell_card(id).unwrap().delivery,
               ShellDelivery::Shared);
    assert!(state.pending_shell_results().is_empty());
    assert_eq!(state.messages.len(),
               1,
               "the entry stays in the conversation");
}

#[test]
fn discarding_clears_the_result_but_keeps_the_entry() {
    let mut state = PanelState::default();
    let id = card(&mut state, "env");
    state.update_shell_command(id, run(ShellStatus::Exited { code: 0 }));

    state.discard_shell_result(id);

    assert_eq!(state.shell_card(id).unwrap().delivery, ShellDelivery::None);
    assert!(state.pending_shell_results().is_empty());
    assert!(matches!(state.messages.first(), Some(PanelMessage::Shell(_))));
}

#[test]
fn discarding_does_not_resurrect_a_shared_result() {
    let mut state = PanelState::default();
    let id = card(&mut state, "ls");
    state.update_shell_command(id, run(ShellStatus::Exited { code: 0 }));
    state.mark_shell_results_shared();

    state.discard_shell_result(id);

    assert_eq!(state.shell_card(id).unwrap().delivery,
               ShellDelivery::Shared);
}

#[test]
fn a_shared_result_is_not_shared_again() {
    let mut state = PanelState::default();
    let id = card(&mut state, "ls");
    state.update_shell_command(id, run(ShellStatus::Exited { code: 0 }));
    state.mark_shell_results_shared();

    // A late poll of the same finished run must not put it back in the queue.
    state.update_shell_command(id, run(ShellStatus::Exited { code: 0 }));

    assert!(state.pending_shell_results().is_empty());
}

#[test]
fn an_update_for_a_card_that_is_gone_is_not_an_error() {
    let mut state = PanelState::default();

    assert!(!state.update_shell_command(Uuid::new_v4(), run(ShellStatus::Exited { code: 0 })));
}
