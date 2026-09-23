//! Signals a shell command's whole process group.
//!
//! Signalling the shell alone is not enough. `!npm test` is a tree, and killing
//! only the `sh` at its root orphans the tree and leaves it holding the pipe
//! the drain threads are reading -- so the run would read as cancelled while
//! its output went on arriving.
//!
//! The group is reached through `kill`, which the crate already requires on
//! `PATH` (see `terminate`), rather than through a new libc dependency.

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
    let _ = run(KILL_PROGRAM, &[signal.flag(), &target]);
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
}
