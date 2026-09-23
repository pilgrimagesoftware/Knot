//! Unit tests for [`super`]: what a row action decides before it touches a
//! process, which is the part that must be right for the rest to be safe.

use std::time::Duration;

use knot_processes::{Activity, DescendantProcess};

use super::{record_termination_failure, termination_target_for};
use crate::agent_processes::ProcessSection;

fn descendant(pid: u32, command: &str) -> DescendantProcess {
    DescendantProcess { pid,
                        ppid: 100,
                        command: command.to_owned(),
                        elapsed: Duration::from_secs(42),
                        activity: Activity::Background }
}

fn sample() -> Vec<DescendantProcess> {
    vec![descendant(200, "npm run dev"),
         descendant(201, "node server.js")]
}

/// Confirming builds a target carrying everything `knot_processes` needs to
/// re-verify the process before it signals anything.
#[test]
fn a_listed_row_yields_a_target_that_names_the_whole_identity() {
    let target = termination_target_for(Some(100), Some(&sample()), 200).unwrap();

    assert_eq!(target.root, 100);
    assert_eq!(target.pid, 200);
    assert_eq!(target.ppid, 100);
    assert_eq!(target.command, "npm run dev");
    assert_eq!(target.elapsed, Duration::from_secs(42));
}

/// A process that exited since the last sample: the spec calls this success,
/// so nothing is asked and nothing is signalled. The row goes on the next
/// sample.
#[test]
fn a_row_older_than_its_process_yields_nothing_to_terminate() {
    assert!(termination_target_for(Some(100), Some(&sample()), 999).is_none());
}

/// An agent that stopped between the render and the click has no session
/// root, so there is no tree to terminate anything within.
#[test]
fn an_agent_with_no_session_root_yields_nothing_to_terminate() {
    assert!(termination_target_for(None, Some(&sample()), 200).is_none());
}

#[test]
fn a_section_with_no_sample_yet_yields_nothing_to_terminate() {
    assert!(termination_target_for(Some(100), None, 200).is_none());
}

/// Cancelling is the absence of the confirm: building the target reads
/// state and changes none of it, so a dialog the user dismisses leaves the
/// section exactly as it was.
#[test]
fn building_a_target_changes_no_state() {
    let mut section = ProcessSection::default();
    section.publish(sample());
    let before = section.clone();

    let _ = termination_target_for(Some(100), section.processes(), 200);

    assert_eq!(section, before);
    assert!(!section.is_terminating(200), "nothing was signalled");
}

/// A refused signal reports by key and leaves the list alone: the process is
/// still running, and no sample will ever remove it.
#[test]
fn a_refused_signal_reports_a_localized_failure_and_keeps_the_row() {
    let mut section = ProcessSection::default();
    section.publish(sample());
    section.mark_terminating(200);

    record_termination_failure(&mut section, 200, "Operation not permitted");

    let failure = section.failure().expect("the refusal is reported");
    assert_eq!(failure,
               knot_core::l10n::t_with("processes.terminate_failed",
                                       &[("reason", "Operation not permitted")]));
    assert!(!failure.contains("%{"), "{failure}");
    assert_ne!(failure, "processes.terminate_failed", "the key resolves");

    assert_eq!(section.processes().map(<[_]>::len),
               Some(2),
               "the row survives");
    assert!(!section.is_terminating(200),
            "no sample will clear this one, so the failure must");
}
