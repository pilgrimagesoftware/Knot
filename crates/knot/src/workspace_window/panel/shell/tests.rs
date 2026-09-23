//! The text a `!` command hands the agent.
//!
//! Asserted at this level rather than through the window: reaching
//! `take_panel_shell_context` needs a live ACP session, and what the
//! requirement is actually about is what the agent is told - which command
//! ran, where, what each stream said, and how it ended.

use knot_processes::{ShellRunState, ShellStatus, ShellStream};
use uuid::Uuid;

use super::shell_context_block;
use crate::panel_state::{PanelState, ShellCard};

fn finished(command: &str, stdout: &str, stderr: &str, status: ShellStatus) -> ShellCard {
    let mut run = ShellRunState::new(1024);
    run.append(ShellStream::Stdout, stdout);
    run.append(ShellStream::Stderr, stderr);
    run.status = status;

    let mut card = ShellCard::starting(Uuid::new_v4(),
                                       command.to_owned(),
                                       "/tmp/worktree".to_owned(),
                                       ShellRunState::new(1024));
    card.absorb(run);
    card
}

#[test]
fn the_block_says_what_ran_where_and_what_it_produced() {
    let card = finished("git status",
                        "nothing to commit\n",
                        "",
                        ShellStatus::Exited { code: 0 });

    let block = shell_context_block(&card);

    assert!(block.contains("/tmp/worktree"), "{block}");
    assert!(block.contains("$ git status"), "{block}");
    assert!(block.contains("nothing to commit"), "{block}");
    assert!(block.contains("(exit 0)"), "{block}");
}

/// The agent has to be able to tell the two streams apart: a command that
/// writes a warning to stderr and still succeeds reads very differently from
/// one that failed.
#[test]
fn stderr_is_labelled_separately() {
    let card = finished("cargo build",
                        "ok\n",
                        "warning: unused\n",
                        ShellStatus::Exited { code: 0 });

    let block = shell_context_block(&card);

    assert!(block.contains("stderr:"), "{block}");
    assert!(block.contains("warning: unused"), "{block}");
}

#[test]
fn a_failing_command_carries_its_code() {
    let card = finished("cargo test",
                        "",
                        "3 failed\n",
                        ShellStatus::Exited { code: 101 });

    assert!(shell_context_block(&card).contains("(exit 101)"));
}

/// Without this the agent reads a truncated head as the whole output and
/// concludes the command produced nothing more.
#[test]
fn truncation_is_stated() {
    let mut run = ShellRunState::new(4);
    run.append(ShellStream::Stdout, "far too long to fit");
    run.status = ShellStatus::Exited { code: 0 };

    let mut card = ShellCard::starting(Uuid::new_v4(),
                                       "ls".to_owned(),
                                       "/tmp".to_owned(),
                                       ShellRunState::new(4));
    card.absorb(run);

    assert!(shell_context_block(&card).contains("truncated"));
}

#[test]
fn a_command_with_no_output_still_reports_its_status() {
    let card = finished("true", "", "", ShellStatus::Exited { code: 0 });

    let block = shell_context_block(&card);

    assert!(block.contains("$ true"), "{block}");
    assert!(block.contains("(exit 0)"), "{block}");
    assert!(!block.contains("stderr:"), "{block}");
}

/// Several commands concatenate in submission order, which is what
/// `take_panel_shell_context` collects.
#[test]
fn blocks_concatenate_in_submission_order() {
    let mut state = PanelState::default();
    for command in ["git status", "git diff"] {
        state.push_shell_command(finished(command, "", "", ShellStatus::Exited { code: 0 }));
    }

    let context: String = state.pending_shell_results()
                               .iter()
                               .map(|card| shell_context_block(card))
                               .collect();

    let status = context.find("git status").expect("first command present");
    let diff = context.find("git diff").expect("second command present");
    assert!(status < diff, "submission order, not completion order");
}

#[test]
fn nothing_pending_produces_no_text() {
    let state = PanelState::default();

    let context: String = state.pending_shell_results()
                               .iter()
                               .map(|card| shell_context_block(card))
                               .collect();

    assert!(context.is_empty(),
            "a prompt with no command before it is unchanged");
}
