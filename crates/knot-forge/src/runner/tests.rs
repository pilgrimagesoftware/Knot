//! Unit tests for [`super`].

use std::time::Duration;

use super::{ForgeRunner, GhRunner};
use crate::error::ForgeError;

/// The one failure that is not a failure: a machine without the binary is a
/// supported configuration, so it has to be distinguishable from every other
/// way spawning can fail.
#[test]
fn a_missing_binary_is_reported_as_missing_not_as_io() {
    let runner = GhRunner::new().with_program("knot-forge-no-such-binary");

    match runner.run(&["auth", "status"]) {
        Err(ForgeError::Missing) => {}
        other => panic!("expected Missing, got {other:?}"),
    }
}

#[test]
fn timeout_kills_process_and_names_command() {
    let runner = GhRunner::new().with_program("sleep")
                                .with_timeout(Duration::from_millis(50));

    let started = std::time::Instant::now();
    let err = runner.run(&["5"]).unwrap_err();

    assert!(started.elapsed() < Duration::from_secs(2),
            "did not abort early");
    match err {
        ForgeError::Timeout { command } => assert_eq!(command, "5"),
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn a_non_zero_exit_carries_stderr_and_the_code() {
    let runner = GhRunner::new().with_program("sh");

    let err = runner.run(&["-c", "echo boom >&2; exit 3"]).unwrap_err();

    match err {
        ForgeError::Command { output, code, .. } => {
            assert_eq!(output, "boom");
            assert_eq!(code, 3);
        }
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn stdout_comes_back_trimmed() {
    let runner = GhRunner::new().with_program("sh");

    let out = runner.run(&["-c", "printf '  hello\\n\\n'"]).unwrap();

    assert_eq!(out, "hello");
}
