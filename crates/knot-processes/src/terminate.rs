//! Ends one listed descendant: `TERM`, a bounded grace period, then `KILL`.
//!
//! The sequence re-reads the process table and re-verifies the target's
//! identity before it signals anything. A PID freed between the sample a row
//! was built from and the moment the user confirms can already belong to an
//! unrelated process, and the only thing standing between that and signalling
//! a stranger is this check.

use std::time::{Duration, Instant};

use crate::command::run;
use crate::consts::{
    KILL_PROGRAM, PS_PROGRAM, PS_STATE_ARGS, SIGNAL_KILL, SIGNAL_TERM, STATE_ZOMBIE,
    TERMINATE_GRACE, TERMINATE_POLL_INTERVAL,
};
use crate::error::{ProcessError, Result};
use crate::sample::sample;
use crate::table::ProcessTable;

/// The floor on the poll interval, so a short grace period in a test does not
/// turn the wait into a busy loop.
const MIN_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Everything a row carries that identifies the process it was built from.
///
/// Held by value rather than borrowed from the snapshot: the sequence runs on
/// a blocking task, outliving the snapshot the row came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminationTarget {
    /// The agent's session root. The target must still descend from it.
    pub root:    u32,
    pub pid:     u32,
    pub ppid:    u32,
    pub command: String,
    /// Runtime as of the sample the row was built from. The same process can
    /// only have run longer since; a recycled PID will have run less.
    pub elapsed: Duration,
}

/// How the sequence ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    /// Exited on `TERM`.
    Terminated,
    /// Outlived the grace period and was sent `KILL`.
    Killed,
    /// Already gone before anything was signalled. Not an error: the row was
    /// simply older than the process.
    AlreadyExited,
}

/// Terminates `target`, escalating to `KILL` after [`TERMINATE_GRACE`].
pub fn terminate(target: &TerminationTarget) -> Result<Termination> {
    terminate_with_grace(target, TERMINATE_GRACE)
}

pub fn terminate_with_grace(target: &TerminationTarget, grace: Duration) -> Result<Termination> {
    match verify(&sample()?, target) {
        Identity::Gone => return Ok(Termination::AlreadyExited),
        Identity::Mismatch => return Err(ProcessError::IdentityMismatch { pid: target.pid }),
        Identity::Match => {}
    }

    if let Some(outcome) = signal(target.pid, SIGNAL_TERM)? {
        return Ok(outcome);
    }

    let poll = TERMINATE_POLL_INTERVAL.min(grace / 4)
                                      .max(MIN_POLL_INTERVAL);
    let deadline = Instant::now() + grace;

    while Instant::now() < deadline {
        if !is_alive(target.pid)? {
            return Ok(Termination::Terminated);
        }

        std::thread::sleep(poll);
    }

    if !is_alive(target.pid)? {
        return Ok(Termination::Terminated);
    }

    match signal(target.pid, SIGNAL_KILL)? {
        Some(outcome) => Ok(outcome),
        None => Ok(Termination::Killed),
    }
}

/// Sends one signal.
///
/// `Ok(None)` means it landed. `Ok(Some(..))` means it was rejected because
/// the process had already gone, which the spec calls success.
fn signal(pid: u32, name: &str) -> Result<Option<Termination>> {
    let pid_text = pid.to_string();
    let output = run(KILL_PROGRAM, &[name, &pid_text])?;

    if output.success {
        return Ok(None);
    }

    // `kill` reports a missing process and a refused signal the same way -- a
    // non-zero exit and a message whose wording is the platform's, not ours.
    // Ask the kernel which it was instead of matching on that text.
    if !is_alive(pid)? {
        return Ok(Some(Termination::AlreadyExited));
    }

    Err(ProcessError::SignalRefused { pid,
                                      output: output.failure_text() })
}

/// Whether the process is still running, as opposed to gone or reaped-pending.
///
/// Asks `ps` for the state letter rather than sending signal `0`, because a
/// zombie -- exited, not yet reaped by its parent -- still accepts signals. A
/// process that exits promptly on `TERM` under a parent that is slow to reap
/// it would otherwise look alive for the whole grace period and be reported as
/// killed.
fn is_alive(pid: u32) -> Result<bool> {
    let pid_text = pid.to_string();
    let mut args = PS_STATE_ARGS.to_vec();
    args.push(&pid_text);

    let output = run(PS_PROGRAM, &args)?;

    if !output.success {
        return Ok(false);
    }

    Ok(!output.stdout.trim().starts_with(STATE_ZOMBIE))
}

#[derive(Debug, PartialEq, Eq)]
enum Identity {
    Match,
    Gone,
    Mismatch,
}

fn verify(table: &ProcessTable, target: &TerminationTarget) -> Identity {
    let Some(record) = table.get(target.pid)
    else {
        return Identity::Gone;
    };

    let same_process = record.ppid == target.ppid
                       && record.command == target.command
                       && record.elapsed >= target.elapsed;

    if same_process && table.is_descendant_of(target.pid, target.root) {
        Identity::Match
    }
    else {
        Identity::Mismatch
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Identity, TerminationTarget, verify};
    use crate::record::ProcessRecord;
    use crate::table::ProcessTable;

    fn record(pid: u32, ppid: u32, command: &str, elapsed: u64) -> ProcessRecord {
        ProcessRecord { pid,
                        ppid,
                        pgid: pid,
                        tpgid: 0,
                        elapsed: Duration::from_secs(elapsed),
                        command: command.to_owned() }
    }

    fn target(command: &str, elapsed: u64) -> TerminationTarget {
        TerminationTarget { root:    100,
                            pid:     200,
                            ppid:    100,
                            command: command.to_owned(),
                            elapsed: Duration::from_secs(elapsed), }
    }

    fn table() -> ProcessTable {
        ProcessTable::from_records([record(100, 1, "shell", 600),
                                    record(200, 100, "sleep 30", 12)])
    }

    #[test]
    fn a_matching_row_verifies() {
        assert_eq!(verify(&table(), &target("sleep 30", 12)), Identity::Match);
    }

    #[test]
    fn a_process_that_has_run_longer_since_the_sample_still_verifies() {
        assert_eq!(verify(&table(), &target("sleep 30", 5)), Identity::Match);
    }

    #[test]
    fn an_absent_pid_is_gone() {
        let empty = ProcessTable::from_records([record(100, 1, "shell", 600)]);

        assert_eq!(verify(&empty, &target("sleep 30", 12)), Identity::Gone);
    }

    #[test]
    fn a_recycled_pid_running_a_different_command_mismatches() {
        assert_eq!(verify(&table(), &target("rm -rf /", 12)),
                   Identity::Mismatch);
    }

    #[test]
    fn a_recycled_pid_with_a_younger_process_mismatches() {
        assert_eq!(verify(&table(), &target("sleep 30", 900)),
                   Identity::Mismatch);
    }

    #[test]
    fn a_process_reparented_out_of_the_agents_tree_mismatches() {
        let reparented = ProcessTable::from_records([record(100, 1, "shell", 600),
                                                     record(200, 1, "sleep 30", 12)]);

        assert_eq!(verify(&reparented, &target("sleep 30", 12)),
                   Identity::Mismatch);
    }

    #[test]
    fn the_session_root_can_never_be_the_target() {
        let root_as_target = TerminationTarget { root:    100,
                                                 pid:     100,
                                                 ppid:    1,
                                                 command: "shell".to_owned(),
                                                 elapsed: Duration::from_secs(600), };

        assert_eq!(verify(&table(), &root_as_target), Identity::Mismatch);
    }
}
