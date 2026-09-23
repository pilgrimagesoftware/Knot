//! These run real subprocesses, deliberately: the behaviours under test -
//! killing on timeout, closing stdin, honouring the working directory and
//! environment - are the ones a stub cannot demonstrate.
//!
//! `/bin/sh` is the program throughout, so nothing here needs an agent CLI
//! installed and the same tests run on both CI legs.

use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::{CommandRunner, McpRunner, ProbeCommand};
use crate::error::ProbeError;

fn sh(script: &str, cwd: &std::path::Path) -> ProbeCommand {
    ProbeCommand::new("/bin/sh", vec!["-c".to_owned(), script.to_owned()], cwd)
}

fn runner_with(timeout: Duration) -> CommandRunner {
    CommandRunner::new().with_timeout(timeout)
}

#[test]
fn stdout_comes_back_on_success() {
    let dir = TempDir::new().unwrap();
    let out = runner_with(Duration::from_secs(10)).run(&sh("echo hello", dir.path()))
                                                  .expect("a successful command");

    assert_eq!(out.trim(), "hello");
}

/// The bound exists so a hung child cannot own the section forever. The
/// assertion that matters is not only the error but the elapsed time: a
/// timeout that did not actually kill the child would still return, late.
#[test]
fn a_hanging_command_is_killed_at_the_timeout() {
    let dir = TempDir::new().unwrap();
    let started = Instant::now();

    let err = runner_with(Duration::from_millis(200)).run(&sh("sleep 30", dir.path()))
                                                     .expect_err("a command that never exits");

    assert!(matches!(err, ProbeError::TimedOut { .. }), "got {err:?}");
    assert!(started.elapsed() < Duration::from_secs(5),
            "the child was not killed; the call took {:?}",
            started.elapsed());
}

/// A CLI that would prompt must fail rather than wait. With stdin inherited
/// this reads from the test harness's own stdin and hangs to the timeout.
#[test]
fn stdin_is_closed_so_a_prompting_command_does_not_wait() {
    let dir = TempDir::new().unwrap();
    let started = Instant::now();

    let out = runner_with(Duration::from_secs(10)).run(&sh("cat", dir.path()))
                                                  .expect("cat on a closed stdin exits cleanly");

    assert_eq!(out.trim(), "");
    assert!(started.elapsed() < Duration::from_secs(5),
            "cat waited for input, so stdin was not closed");
}

/// Project-scoped MCP configuration resolves against the agent's working
/// directory. A probe run from Knot's own cwd answers a different question.
#[test]
fn the_command_runs_in_the_given_working_directory() {
    let dir = TempDir::new().unwrap();
    let expected = dir.path().file_name().unwrap().to_str().unwrap().to_owned();

    let out = runner_with(Duration::from_secs(10)).run(&sh("pwd", dir.path()))
                                                  .expect("pwd");

    assert!(out.trim().ends_with(&expected),
            "ran in {out:?}, expected a directory ending {expected:?}");
}

/// `CLAUDE_CONFIG_DIR` and friends decide which user configuration is in
/// play, so the agent's launch environment has to reach the child.
#[test]
fn the_agents_environment_reaches_the_child() {
    let dir = TempDir::new().unwrap();
    let command =
        sh("echo $KNOT_PROBE_MARKER", dir.path()).with_env(vec![("KNOT_PROBE_MARKER".to_owned(),
                                                                 "carried".to_owned())]);

    let out = runner_with(Duration::from_secs(10)).run(&command)
                                                  .expect("echo");

    assert_eq!(out.trim(), "carried");
}

#[test]
fn a_missing_program_is_reported_as_missing() {
    let dir = TempDir::new().unwrap();
    let command = ProbeCommand::new("knot-no-such-agent-cli", vec!["mcp".to_owned()], dir.path());

    let err = runner_with(Duration::from_secs(10)).run(&command)
                                                  .expect_err("a program that is not installed");

    assert!(matches!(&err, ProbeError::Missing { program } if program == "knot-no-such-agent-cli"),
            "got {err:?}");
}

/// The case that would otherwise discard the answer. `claude mcp list` exits
/// non-zero when a server fails its health check *and still lists every
/// server* - which is precisely the state the section exists to show. Judging
/// on the exit code alone would turn "one server is down" into "the probe
/// failed".
#[test]
fn a_nonzero_exit_that_still_listed_is_not_a_failure() {
    let dir = TempDir::new().unwrap();
    let out = runner_with(Duration::from_secs(10))
        .run(&sh("echo 'github: https://x - connected'; exit 1", dir.path()))
        .expect("output survives a non-zero exit");

    assert!(out.contains("github"));
}

#[test]
fn a_nonzero_exit_with_nothing_on_stdout_is_a_failure() {
    let dir = TempDir::new().unwrap();
    let err = runner_with(Duration::from_secs(10)).run(&sh("echo 'not logged in' >&2; exit 3",
                                                           dir.path()))
                                                  .expect_err("a command that only failed");

    match err {
        ProbeError::Command { output, code, .. } => {
            assert_eq!(code, 3);
            assert!(output.contains("not logged in"),
                    "stderr is the message when stdout is empty");
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn a_label_names_the_whole_command() {
    let dir = TempDir::new().unwrap();
    let command = ProbeCommand::new("claude",
                                    vec!["mcp".to_owned(), "list".to_owned()],
                                    dir.path());

    assert_eq!(command.label(), "claude mcp list");
    assert_eq!(ProbeCommand::new("claude", vec![], dir.path()).label(),
               "claude");
}
