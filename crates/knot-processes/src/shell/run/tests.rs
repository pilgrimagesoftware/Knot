//! Exercises the runner against real processes.
//!
//! Every test here starts a child, so each one waits on something. They wait
//! on the run's own terminal status rather than on a sleep: a fixed sleep is
//! either slower than it needs to be or flaky on a loaded machine, and these
//! run in CI.

use std::ffi::OsString;
use std::path::Path;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use super::spawn;
use crate::shell::invocation::ShellRequest;
use crate::shell::status::ShellStatus;

/// How long any one test will wait for a command that should finish promptly.
/// Generous: it bounds a hang, it does not pace the test.
const SETTLE_TIMEOUT: Duration = Duration::from_secs(30);

fn request(command: &str, cwd: &Path) -> ShellRequest {
    let mut request = ShellRequest::new(command, cwd);
    // Not the runner's `SHELL`: a developer's login shell may print a banner
    // or take half a second to source a profile, and neither belongs in an
    // assertion about what a command wrote.
    request.shell = Some(OsString::from("/bin/sh"));
    request
}

/// Waits for the run to settle, returning its final state.
fn settled(run: &crate::shell::ShellRun) -> crate::shell::ShellRunState {
    let deadline = Instant::now() + SETTLE_TIMEOUT;

    while !run.is_finished() {
        assert!(Instant::now() < deadline,
                "run did not settle within {SETTLE_TIMEOUT:?}");
        std::thread::sleep(Duration::from_millis(5));
    }

    run.snapshot()
}

#[test]
fn a_command_runs_and_reports_its_output() {
    let dir = tempdir().unwrap();

    let state = settled(&spawn(request("echo hi", dir.path())));

    assert_eq!(state.status, ShellStatus::Exited { code: 0 });
    assert_eq!(state.stdout.text().trim(), "hi");
    assert!(state.stderr.is_empty());
}

#[test]
fn a_command_runs_in_the_folder_it_was_given() {
    let dir = tempdir().unwrap();
    // The temp dir may be a symlink (`/var` -> `/private/var` on macOS), and
    // `pwd` reports the resolved path, so compare resolved paths.
    let expected = dir.path().canonicalize().unwrap();

    let state = settled(&spawn(request("pwd", dir.path())));

    assert_eq!(Path::new(state.stdout.text().trim()), expected);
}

#[test]
fn a_failing_command_reports_its_exit_status_and_stderr() {
    let dir = tempdir().unwrap();

    let state = settled(&spawn(request("echo out; echo err >&2; exit 3", dir.path())));

    assert_eq!(state.status, ShellStatus::Exited { code: 3 });
    assert!(state.status.is_failure());
    assert_eq!(state.stdout.text().trim(), "out");
    assert_eq!(state.stderr.text().trim(), "err");
}

#[test]
fn each_stream_is_captured_separately() {
    let dir = tempdir().unwrap();

    let state = settled(&spawn(request("echo only-err >&2", dir.path())));

    assert!(state.stdout.is_empty(), "stderr must not reach stdout");
    assert_eq!(state.stderr.text().trim(), "only-err");
}

#[test]
fn a_command_that_reads_input_sees_end_of_file() {
    let dir = tempdir().unwrap();

    // Without a closed stdin this waits forever and the test times out.
    let state = settled(&spawn(request("cat; echo done", dir.path())));

    assert_eq!(state.status, ShellStatus::Exited { code: 0 });
    assert_eq!(state.stdout.text().trim(), "done");
}

#[test]
fn a_directory_change_does_not_outlive_its_command() {
    let dir = tempdir().unwrap();
    let expected = dir.path().canonicalize().unwrap();

    settled(&spawn(request("cd ..", dir.path())));
    let state = settled(&spawn(request("pwd", dir.path())));

    assert_eq!(Path::new(state.stdout.text().trim()),
               expected,
               "each command starts in the folder it was given");
}

#[test]
fn a_multi_line_command_runs_every_line() {
    let dir = tempdir().unwrap();

    let state = settled(&spawn(request("echo first\necho second", dir.path())));

    assert_eq!(state.stdout.text().trim(), "first\nsecond");
}

#[test]
fn output_past_the_limit_is_truncated_at_the_head() {
    let dir = tempdir().unwrap();
    let mut request = request("printf 'abcdefghij'", dir.path());
    request.output_limit = 4;

    let state = settled(&spawn(request));

    assert_eq!(state.stdout.text(), "abcd");
    assert!(state.stdout.is_truncated());
    assert!(state.is_truncated());
    assert_eq!(state.status,
               ShellStatus::Exited { code: 0 },
               "truncating capture does not end the command");
}

#[test]
fn a_command_still_running_at_the_deadline_times_out() {
    let dir = tempdir().unwrap();
    let mut request = request("echo starting; sleep 30", dir.path());
    request.timeout = Duration::from_millis(100);

    let started = Instant::now();
    let state = settled(&spawn(request));

    assert_eq!(state.status, ShellStatus::TimedOut);
    assert!(started.elapsed() < SETTLE_TIMEOUT, "did not abort early");
    assert_eq!(state.stdout.text().trim(),
               "starting",
               "output captured before the deadline survives");
}

#[test]
fn cancelling_ends_the_command_and_keeps_what_it_wrote() {
    let dir = tempdir().unwrap();
    let run = spawn(request("echo started; sleep 30", dir.path()));

    // Cancel only once the command has actually produced something, so the
    // assertion about surviving output is about capture and not about timing.
    let deadline = Instant::now() + SETTLE_TIMEOUT;
    while run.snapshot().stdout.is_empty() {
        assert!(Instant::now() < deadline, "command produced no output");
        std::thread::sleep(Duration::from_millis(5));
    }

    run.cancel();
    let state = settled(&run);

    assert_eq!(state.status, ShellStatus::Cancelled);
    assert_eq!(state.stdout.text().trim(), "started");
}

#[test]
fn cancelling_reaches_the_whole_process_group() {
    let dir = tempdir().unwrap();
    // `sh` waits on a child it backgrounded. Signalling only `sh` would leave
    // the `sleep` holding the pipe, and this run would never settle.
    let run = spawn(request("sleep 30 & wait", dir.path()));

    // Give the tree a moment to exist before signalling it.
    std::thread::sleep(Duration::from_millis(100));
    run.cancel();

    assert_eq!(settled(&run).status, ShellStatus::Cancelled);
}

#[test]
fn cancelling_one_run_leaves_another_alone() {
    let dir = tempdir().unwrap();
    let cancelled = spawn(request("sleep 30", dir.path()));
    let untouched = spawn(request("sleep 0.2; echo survived", dir.path()));

    cancelled.cancel();

    assert_eq!(settled(&cancelled).status, ShellStatus::Cancelled);
    let state = settled(&untouched);
    assert_eq!(state.status, ShellStatus::Exited { code: 0 });
    assert_eq!(state.stdout.text().trim(), "survived");
}

#[test]
fn the_dirty_flag_reports_a_change_once() {
    let dir = tempdir().unwrap();
    let run = spawn(request("echo hi", dir.path()));

    settled(&run);

    assert!(run.take_dirty(),
            "output and a final status are both changes");
    assert!(!run.take_dirty(), "the flag clears as it is read");
}

#[test]
fn a_shell_that_cannot_be_launched_fails_to_start() {
    let dir = tempdir().unwrap();
    let mut request = request("echo hi", dir.path());
    request.shell = Some(OsString::from("/knot-no-such-shell"));

    let state = settled(&spawn(request));

    match state.status {
        ShellStatus::FailedToStart { message } => {
            assert!(!message.is_empty(), "the OS reason is carried");
        }
        other => panic!("expected FailedToStart, got {other:?}"),
    }
    assert!(state.stdout.is_empty());
}

#[test]
fn a_missing_folder_fails_to_start_rather_than_running_elsewhere() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("no-such-folder");

    let state = settled(&spawn(request("pwd", &missing)));

    assert!(matches!(state.status, ShellStatus::FailedToStart { .. }),
            "got {:?}",
            state.status);
}
