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

/// The bug this crate had: a Finder-launched app's `PATH` names no
/// directory `gh` is ever installed in, so spawning it by bare name failed
/// and the view reported the tool missing on a machine that has it.
#[test]
fn gh_is_located_in_an_install_directory_the_process_path_omits() {
    let dir = tempfile::tempdir().expect("temp dir");
    let installed = write_executable(dir.path(), "gh");

    let located = super::locate_gh(&dir.path().display().to_string());

    assert_eq!(located, installed,
               "gh must be located by absolute path, not left to the spawn's own lookup");
}

#[test]
fn a_path_holding_no_gh_falls_back_to_the_bare_name() {
    let dir = tempfile::tempdir().expect("temp dir");

    let located = super::locate_gh(&dir.path().display().to_string());

    assert_eq!(located,
               std::ffi::OsString::from("gh"),
               "a machine without gh must still reach the NotFound spawn that reports Missing");
}

#[test]
fn the_located_binary_is_what_runs() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_script(dir.path(), "gh", "printf 'from the located binary'");

    let runner = GhRunner::new().with_program(super::locate_gh(&dir.path().display().to_string()));

    assert_eq!(runner.run(&["auth", "status"]).unwrap(),
               "from the located binary");
}

#[test]
fn the_child_is_given_the_merged_search_path() {
    let runner = GhRunner::new().with_program("sh");

    let child_path = runner.run(&["-c", "printf '%s' \"$PATH\""]).unwrap();

    assert_eq!(child_path,
               runner.search_path().to_string_lossy(),
               "gh shells out to git and to credential helpers, so it needs the merged path too");
    assert!(child_path.split(':')
                      .any(|entry| entry == "/opt/homebrew/bin"),
            "the merged path must name the standard install locations: {child_path}");
}

fn write_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    write_script(dir, name, "true")
}

#[cfg(unix)]
fn write_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

#[cfg(not(unix))]
fn write_script(dir: &std::path::Path, name: &str, _body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, b"").expect("write");
    path
}
