//! Unit tests for [`super`].

use std::fs;

use super::{resolve_program_on, search_path_for};

#[test]
fn process_entries_come_first_and_keep_their_order() {
    let path = search_path_for("/usr/local/bin:/usr/bin", "/Users/tester");
    let entries: Vec<&str> = path.split(':').collect();

    assert_eq!(&entries[..2],
               &["/usr/local/bin", "/usr/bin"],
               "the process's own PATH entries must keep their order and come first");
    assert!(entries.contains(&"/opt/homebrew/bin"));
    assert!(entries.contains(&"/Users/tester/.cargo/bin"));
}

#[test]
fn a_fallback_already_on_the_process_path_is_not_repeated() {
    let path = search_path_for("/opt/homebrew/bin:/usr/bin:/bin", "/Users/tester");
    let entries: Vec<&str> = path.split(':').collect();

    assert_eq!(entries.iter()
                      .filter(|entry| **entry == "/opt/homebrew/bin")
                      .count(),
               1,
               "a fallback already named by the process PATH must not be appended again");
    assert_eq!(entries[0], "/opt/homebrew/bin",
               "and it keeps the process PATH's ordering (first, in this case)");
}

#[test]
fn tilde_fallbacks_expand_against_home() {
    let path = search_path_for("", "/Users/tester");
    let entries: Vec<&str> = path.split(':').collect();

    for expected in ["/Users/tester/.cargo/bin",
                     "/Users/tester/.local/bin",
                     "/Users/tester/.npm-global/bin"]
    {
        assert!(entries.contains(&expected), "missing {expected} in {path}");
    }
    assert!(!entries.iter().any(|entry| entry.starts_with('~')),
            "no literal `~` entry may survive into the merged path");
}

#[test]
fn an_empty_home_keeps_the_fallback_literal() {
    let path = search_path_for("", "");

    assert!(path.split(':').any(|entry| entry == "~/.cargo/bin"),
            "an empty HOME keeps the fallback literal rather than mangling it");
}

/// The bug every caller of this module exists for: the `PATH` a
/// Finder-launched macOS app actually sees.
#[test]
fn the_launchd_gui_path_gains_the_standard_install_locations() {
    let path = search_path_for("/usr/bin:/bin:/usr/sbin:/sbin", "/Users/tester");

    for expected in ["/opt/homebrew/bin",
                     "/usr/local/bin",
                     "/Users/tester/.cargo/bin",
                     "/Users/tester/.local/bin",
                     "/Users/tester/.npm-global/bin"]
    {
        assert!(path.split(':').any(|entry| entry == expected),
                "missing {expected} in {path}");
    }
    assert_eq!(path.split(':').next(),
               Some("/usr/bin"),
               "the launchd entries must still come first when present");
}

#[test]
fn an_executable_resolves_to_the_first_directory_holding_it() {
    let first = tempfile::tempdir().expect("temp dir");
    let second = tempfile::tempdir().expect("temp dir");
    write_executable(first.path(), "knot-test-tool");
    write_executable(second.path(), "knot-test-tool");

    let path = format!("{}:{}", first.path().display(), second.path().display());

    assert_eq!(resolve_program_on(&path, "knot-test-tool"),
               Some(first.path().join("knot-test-tool")),
               "the earlier directory on the search path wins");
}

#[test]
fn a_directory_later_on_the_path_is_still_searched() {
    let empty = tempfile::tempdir().expect("temp dir");
    let holding = tempfile::tempdir().expect("temp dir");
    write_executable(holding.path(), "knot-test-tool");

    let path = format!("{}:{}", empty.path().display(), holding.path().display());

    assert_eq!(resolve_program_on(&path, "knot-test-tool"),
               Some(holding.path().join("knot-test-tool")));
}

#[test]
fn a_non_executable_file_does_not_resolve() {
    let dir = tempfile::tempdir().expect("temp dir");
    fs::write(dir.path().join("knot-test-tool"), b"not runnable").expect("write");

    assert_eq!(resolve_program_on(&dir.path().display().to_string(), "knot-test-tool"),
               None,
               "a readable file that cannot be run is not the tool");
}

#[test]
fn a_program_absent_from_every_directory_does_not_resolve() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert_eq!(resolve_program_on(&dir.path().display().to_string(), "knot-test-tool"),
               None);
}

#[test]
fn a_program_naming_a_file_is_taken_as_given() {
    assert_eq!(resolve_program_on("/usr/bin", "/opt/custom/gh"),
               Some("/opt/custom/gh".into()),
               "a path is a path, not something to look up");
}

#[test]
fn an_empty_program_does_not_resolve() {
    assert_eq!(resolve_program_on("/usr/bin", ""), None);
}

#[cfg(unix)]
fn write_executable(dir: &std::path::Path, name: &str) {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    fs::write(&path, b"#!/bin/sh\n").expect("write");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

#[cfg(not(unix))]
fn write_executable(dir: &std::path::Path, name: &str) {
    fs::write(dir.join(name), b"").expect("write");
}
