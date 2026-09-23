//! Terminates live processes. See `openspec/specs/agent-processes/spec.md` --
//! "A listed process can be terminated".
//!
//! Every test here spawns a real child of the test binary and targets it with
//! this process as the session root, so nothing outside the test's own subtree
//! can be signalled even if an assertion is wrong.

use std::process::{Child, Command};
use std::time::Duration;

use knot_processes::terminate::terminate_with_grace;
use knot_processes::{ProcessError, Termination, TerminationTarget, sample, terminate};

/// Short enough to keep the escalation test quick, long enough that a process
/// that does exit on `TERM` is seen exiting rather than killed by the clock.
const TEST_GRACE: Duration = Duration::from_millis(600);

/// Kills and reaps the child even when an assertion unwinds past it, so a
/// failing test never leaves a `sleep` behind.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Builds the target the way a row does: from a real sample, so the identity
/// check is exercised against values the parser produced.
fn target_for(pid: u32) -> TerminationTarget {
    let root = std::process::id();
    let table = sample().expect("sample the process table");
    let record = table.get(pid).expect("the spawned child is in the table");

    TerminationTarget { root,
                        pid,
                        ppid: record.ppid,
                        command: record.command.clone(),
                        elapsed: record.elapsed }
}

fn is_alive(pid: u32) -> bool {
    sample().expect("sample the process table").contains(pid)
}

#[test]
fn a_process_exits_on_term() {
    let child = Reaped(Command::new("sleep").arg("30")
                                            .spawn()
                                            .expect("spawn sleep"));
    let pid = child.0.id();

    let outcome = terminate_with_grace(&target_for(pid), TEST_GRACE).unwrap();

    assert_eq!(outcome, Termination::Terminated);
}

#[test]
fn a_process_that_ignores_term_is_killed_after_the_grace_period() {
    // `trap "" TERM` makes the shell ignore the signal outright; the loop
    // keeps it alive so there is something for `KILL` to reach.
    let child = Reaped(Command::new("sh").arg("-c")
                                         .arg("trap '' TERM; while :; do sleep 0.1; done")
                                         .spawn()
                                         .expect("spawn a TERM-ignoring shell"));
    let pid = child.0.id();

    let outcome = terminate_with_grace(&target_for(pid), TEST_GRACE).unwrap();

    assert_eq!(outcome, Termination::Killed);
}

#[test]
fn an_already_exited_process_is_reported_as_success() {
    let mut child = Command::new("sleep").arg("30")
                                         .spawn()
                                         .expect("spawn sleep");
    let pid = child.id();
    let target = target_for(pid);

    child.kill().expect("kill the child");
    child.wait().expect("reap the child");

    let outcome = terminate_with_grace(&target, TEST_GRACE).unwrap();

    assert_eq!(outcome, Termination::AlreadyExited);
}

#[test]
fn a_mismatched_identity_is_refused_without_signalling() {
    let child = Reaped(Command::new("sleep").arg("30")
                                            .spawn()
                                            .expect("spawn sleep"));
    let pid = child.0.id();

    let mut target = target_for(pid);
    target.command = "something else entirely".to_owned();

    let err = terminate_with_grace(&target, TEST_GRACE).unwrap_err();

    assert!(matches!(err, ProcessError::IdentityMismatch { pid: reported } if reported == pid));
    assert!(is_alive(pid), "nothing was signalled");
}

#[test]
fn a_process_outside_the_session_root_is_refused() {
    let child = Reaped(Command::new("sleep").arg("30")
                                            .spawn()
                                            .expect("spawn sleep"));
    let sibling = Reaped(Command::new("sleep").arg("30")
                                              .spawn()
                                              .expect("spawn sleep"));
    let pid = child.0.id();

    let mut target = target_for(pid);
    // A sibling, not an ancestor: the row now claims a session root the target
    // does not descend from. (PID 1 would not do -- it is every process's
    // ancestor, so the check would rightly pass.)
    target.root = sibling.0.id();

    let err = terminate_with_grace(&target, TEST_GRACE).unwrap_err();

    assert!(matches!(err, ProcessError::IdentityMismatch { .. }));
    assert!(is_alive(pid), "nothing was signalled");
}

#[test]
fn terminating_one_child_leaves_its_sibling_running() {
    let doomed = Reaped(Command::new("sleep").arg("30")
                                             .spawn()
                                             .expect("spawn sleep"));
    let spared = Reaped(Command::new("sleep").arg("30")
                                             .spawn()
                                             .expect("spawn sleep"));
    let doomed_pid = doomed.0.id();
    let spared_pid = spared.0.id();

    let outcome = terminate_with_grace(&target_for(doomed_pid), TEST_GRACE).unwrap();

    assert_eq!(outcome, Termination::Terminated);
    assert!(is_alive(spared_pid), "the sibling kept running");
    assert!(is_alive(std::process::id()),
            "the session root kept running");
}

#[test]
fn the_default_grace_period_terminates_a_cooperative_process() {
    let child = Reaped(Command::new("sleep").arg("30")
                                            .spawn()
                                            .expect("spawn sleep"));
    let pid = child.0.id();

    assert_eq!(terminate(&target_for(pid)).unwrap(),
               Termination::Terminated);
}
