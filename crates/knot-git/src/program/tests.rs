//! Unit tests for [`super`].
//!
//! Only what can be asserted without touching the process-wide value: the
//! set-once behavior itself is exercised in `tests/program.rs`, which gets a
//! process of its own. A unit test that configured this binary's value would
//! change what every other test in it spawns.

use std::ffi::OsString;

use super::configured;

#[test]
fn an_unconfigured_process_runs_the_bare_name() {
    let default = configured();

    assert_eq!(default.program, OsString::from("git"));
    assert!(default.search_path.is_none(),
            "nothing set means the child inherits this process's PATH, as it always did");
}
