//! The configured git, end to end.
//!
//! One test in its own file on purpose. [`knot_git::configure`] is
//! set-once and process-wide, so a second test in this binary would either
//! race this one for the first call or observe whichever ran first.
//! Separate integration binaries are what makes the ordering a fact rather
//! than a hope.

use std::ffi::OsString;
use std::process::Command;

use knot_git::program::configured;
use knot_git::{GitProgram, Runner, configure};

#[test]
fn the_configured_git_is_what_runners_spawn() {
    let dir = tempfile::tempdir().expect("temp dir");
    let git = absolute_git();
    let search_path = OsString::from("/usr/bin:/bin:/opt/homebrew/bin");

    // Unconfigured, the crate runs what it always ran.
    assert_eq!(configured().program,
               OsString::from("git"),
               "the default must stay the bare name, for every caller that never configures");
    assert!(configured().search_path.is_none(),
            "and must leave the child's PATH inherited");

    assert!(configure(GitProgram { program:     git.clone(),
                                   search_path: Some(search_path.clone()), }),
            "the first call must take");
    assert_eq!(configured().program, git);
    assert_eq!(configured().search_path, Some(search_path));

    // Set-once: a later caller cannot leave two runners disagreeing about
    // which git they mean.
    assert!(!configure(GitProgram { program:     OsString::from("knot-git-no-such-binary"),
                                    search_path: None, }),
            "a second call must report that it did nothing");
    assert_eq!(configured().program,
               git,
               "and must not have changed anything");

    // A runner built now picks it up without being told.
    let version = Runner::new(dir.path()).run(&["--version"])
                                         .expect("the configured git runs");
    assert!(version.starts_with("git version"),
            "unexpected output: {version}");
}

/// The absolute path to this machine's git, so the test proves a located
/// binary is spawnable rather than re-proving that `PATH` lookup works.
fn absolute_git() -> OsString {
    let out = Command::new("sh").args(["-c", "command -v git"])
                                .output()
                                .expect("locate git");
    let path = String::from_utf8_lossy(&out.stdout).trim().to_owned();

    assert!(path.starts_with('/'),
            "these tests need a git on PATH; found {path:?}");
    OsString::from(path)
}
