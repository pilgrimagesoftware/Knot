//! Naming the external tools this process runs, once, before the app starts.
//!
//! Knot is a GUI app: launched from Finder it inherits launchd's
//! `/usr/bin:/bin:/usr/sbin:/sbin`, and every tool installed by Homebrew,
//! cargo or npm is invisible to it. `knot_core::exec_path` knows where to
//! look; the crates that spawn the tools deliberately do not depend on it
//! (`knot-git` is standalone by design - see the workspace layout in
//! `AGENTS.md`), so the binary is where the two meet.
//!
//! Called from `main` rather than from `app_bootstrap`, because it has to
//! happen before anything builds a runner and because it is about this
//! process's environment rather than about the application's windows.

use std::ffi::OsString;
use std::path::PathBuf;

/// Points `knot-git` at a located `git` and the merged search path.
pub(crate) fn configure() {
    knot_git::configure(git_program(knot_core::exec_path::resolve_program(knot_git::consts::GIT_PROGRAM),
                                    knot_core::exec_path::search_path()));
}

/// Pure form of [`configure`]'s decision.
///
/// An unresolved `git` keeps the bare name rather than becoming an error:
/// `/usr/bin/git` is on every macOS `PATH` already, so the located path is
/// an improvement on the common case and a rescue for the uncommon one -
/// not a precondition for the app starting.
fn git_program(resolved: Option<PathBuf>, search_path: String) -> knot_git::GitProgram {
    knot_git::GitProgram { program:     resolved.map_or_else(|| {
                                                    OsString::from(knot_git::consts::GIT_PROGRAM)
                                                },
                                                OsString::from),
                           search_path: Some(OsString::from(search_path)), }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_located_git_is_the_one_configured() {
        let program = git_program(Some(PathBuf::from("/opt/homebrew/bin/git")),
                                  "/usr/bin:/opt/homebrew/bin".to_owned());

        assert_eq!(program.program, OsString::from("/opt/homebrew/bin/git"));
    }

    #[test]
    fn an_unlocated_git_falls_back_to_the_bare_name() {
        let program = git_program(None, "/usr/bin".to_owned());

        assert_eq!(program.program,
                   OsString::from("git"),
                   "the app must still start, and the spawn's own lookup is what it always used");
    }

    #[test]
    fn the_merged_search_path_is_carried_either_way() {
        for resolved in [Some(PathBuf::from("/opt/homebrew/bin/git")), None] {
            let program = git_program(resolved, "/usr/bin:/opt/homebrew/bin".to_owned());

            assert_eq!(program.search_path,
                       Some(OsString::from("/usr/bin:/opt/homebrew/bin")),
                       "git's own helpers need the merged path whether or not git itself moved");
        }
    }
}
