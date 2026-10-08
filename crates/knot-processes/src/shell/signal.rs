//! Signals a shell command's whole process group.
//!
//! Signalling the shell alone is not enough. `!npm test` is a tree, and killing
//! only the `sh` at its root orphans the tree and leaves it holding the pipe
//! the drain threads are reading -- so the run would read as cancelled while
//! its output went on arriving.
//!
//! The group is reached through `kill`, which the crate already requires on
//! `PATH` (see `terminate`), rather than through a new libc dependency.
//!
//! The negative group id goes after `--`. procps-ng's `kill` - the
//! `/usr/bin/kill` on Ubuntu and most Linux distributions - parses its
//! arguments with `getopt`, so a bare `-<pgid>` is read as an option, and its
//! "signal digit" rule for negative pids turns it into `-<first digit>`. Any
//! group id starting with `1` therefore became `kill -TERM -1`: every process
//! the user owns. That killed the GitHub Actions runner during the v1.24.0
//! release, and would have ended a Linux user's whole session on cancelling a
//! `!` command. `--` ends option parsing, so the target is taken as written;
//! macOS's BSD `kill` accepts the same form.

use crate::command::run;
use crate::consts::{KILL_PROGRAM, SIGNAL_KILL, SIGNAL_TERM};

/// Whether the signal is the polite one or the final one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Term,
    Kill,
}

impl Signal {
    fn flag(self) -> &'static str {
        match self {
            Self::Term => SIGNAL_TERM,
            Self::Kill => SIGNAL_KILL,
        }
    }
}

/// Signals the process group led by `pid`.
///
/// The child is spawned into its own group, so the group id is the child's own
/// pid, and `kill` names a group by negating it.
///
/// A `pid` of 0 or 1 is refused. Negated, those are `kill`'s two wildcards:
/// `-0` is the caller's *own* group -- which is the application -- and `-1` is
/// every process the user is allowed to signal. Neither can be a child's group
/// id, so refusing them costs nothing and removes the only way this function
/// could reach something it was not given.
///
/// Failure is otherwise not reported: by the time the supervisor signals, the
/// only reasons this fails are that the group has already exited or that it
/// never started, and neither changes what the run does next -- it goes on
/// polling for the exit either way.
pub fn signal_group(pid: u32, signal: Signal) {
    if pid <= 1 {
        return;
    }

    let target = format!("-{pid}");
    // `--` before the target: without it procps-ng `kill` parses `-<pgid>`
    // as an option and can signal `-1` instead - see the module doc.
    let _ = run(KILL_PROGRAM, &[signal.flag(), "--", &target]);
}

#[cfg(test)]
mod tests {
    use super::{Signal, signal_group};
    use crate::consts::{SIGNAL_KILL, SIGNAL_TERM};

    #[test]
    fn each_signal_maps_to_its_flag() {
        assert_eq!(Signal::Term.flag(), SIGNAL_TERM);
        assert_eq!(Signal::Kill.flag(), SIGNAL_KILL);
    }

    #[test]
    fn the_wildcard_groups_are_refused() {
        // `kill -TERM -1` signals every process the user owns and `kill -TERM
        // -0` signals this one's own group. Neither is reachable, and this
        // test asserts it by running both: it is the whole test suite that
        // fails if the guard goes.
        signal_group(0, Signal::Term);
        signal_group(1, Signal::Kill);
    }

    /// Signalling one group reaches that group and nothing else (hotfix
    /// 1.24.1). A sibling started separately, in a group of its own, must
    /// survive the cancel.
    ///
    /// Without `--`, procps-ng `kill` turned any group id starting with `1`
    /// into `-1` - every process the user owns - which is what killed the CI
    /// runner. That only happens on Linux, and only for such ids, so on macOS
    /// this passes either way; on Linux CI it is the guard.
    #[test]
    fn signalling_a_group_spares_an_unrelated_process() {
        use std::os::unix::process::CommandExt;
        use std::process::{Child, Command};
        use std::time::{Duration, Instant};

        fn own_group_sleep() -> Child {
            Command::new("sleep").arg("30")
                                 .process_group(0)
                                 .spawn()
                                 .expect("sleep starts")
        }
        fn exited_within(child: &mut Child, limit: Duration) -> bool {
            let started = Instant::now();
            while started.elapsed() < limit {
                if child.try_wait().expect("try_wait works").is_some() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        }

        let mut target = own_group_sleep();
        let mut sibling = own_group_sleep();

        signal_group(target.id(), Signal::Term);

        let target_exited = exited_within(&mut target, Duration::from_secs(5));
        let sibling_alive = sibling.try_wait().expect("try_wait works").is_none();
        let _ = sibling.kill();
        let _ = sibling.wait();
        let _ = target.kill();
        let _ = target.wait();
        assert!(target_exited, "the signalled group did not exit");
        assert!(sibling_alive,
                "signalling one group killed an unrelated process");
    }
}
