use std::time::Duration;

/// Wall-clock ceiling on one `ps` or `kill` invocation. A wedged child would
/// otherwise hold a blocking task for the life of the process, once per
/// sampling interval.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

pub const PS_PROGRAM: &str = "ps";

/// Every process, six columns, no headers. `command` is last because it is the
/// only column that contains spaces, so the parser can split a fixed number of
/// leading fields and take the remainder verbatim.
pub const PS_ARGS: &[&str] = &["-Ao", "pid=,ppid=,pgid=,tpgid=,etime=,command="];

/// Leading whitespace-delimited fields before the command column.
pub const PS_LEADING_FIELDS: usize = 5;

/// Asks `ps` for one process's state letter. Completed by the PID.
///
/// `kill -0` cannot answer the liveness question on its own: a process that
/// has exited but whose parent has not reaped it is a zombie, and a zombie
/// still accepts signals. Polling with `kill -0` would therefore wait out the
/// whole grace period and escalate to `KILL` for a process that did exactly
/// what `TERM` asked of it.
pub const PS_STATE_ARGS: &[&str] = &["-o", "state=", "-p"];

/// The state letter `ps` reports for a process that has exited and is waiting
/// to be reaped.
pub const STATE_ZOMBIE: char = 'Z';

pub const KILL_PROGRAM: &str = "kill";

pub const SIGNAL_TERM: &str = "-TERM";
pub const SIGNAL_KILL: &str = "-KILL";

/// How long a process gets to exit on `TERM` before it is sent `KILL`.
pub const TERMINATE_GRACE: Duration = Duration::from_secs(5);

/// How often the termination sequence re-checks whether the target has exited.
/// Each check is a `kill -0` subprocess, and the row stays marked terminating
/// until the next sample anyway, so polling faster than this buys nothing the
/// user can see.
pub const TERMINATE_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// How often an expanded processes section re-reads the process table. One
/// read serves every observed agent in the window, so this is the whole cost
/// of the section being open.
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(3);

/// `ps` reports no controlling terminal as `0` on macOS and `-1` on Linux.
/// Neither can equal a real process group id, so both fall through to
/// background without being special-cased.
pub const NO_FOREGROUND_GROUP: i32 = -1;

/// Wall-clock ceiling on one `!` shell command. Deliberately not
/// [`DEFAULT_TIMEOUT`]: that bounds a `ps` or `kill` that should answer
/// instantly, whereas a test run or a build is a legitimate `!` command and
/// routinely takes minutes.
pub const SHELL_TIMEOUT: Duration = Duration::from_secs(120);

/// Bytes kept from one captured stream before the rest is discarded. The head
/// is what is kept: it is where a command states what it could not do.
pub const SHELL_OUTPUT_LIMIT: usize = 256 * 1024;

/// How often the supervisor re-checks whether the command has exited. Output
/// reaches the screen from the drain threads, not from this loop, so polling
/// faster buys no responsiveness -- only wakeups, once per running command.
pub const SHELL_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// How long a cancelled or timed-out command gets to exit on `TERM` before its
/// process group is sent `KILL`. Shorter than [`TERMINATE_GRACE`]: that grace
/// is a courtesy to an agent that may be writing a file, while this one is a
/// user who has already asked twice by waiting.
pub const SHELL_KILL_GRACE: Duration = Duration::from_secs(2);

/// Runs when `SHELL` is unset or empty.
pub const SHELL_FALLBACK: &str = "/bin/sh";

/// Login shell, reading the command from the next argument. `-l` is what makes
/// `!npm test` find `npm`: the application is launched from Finder with an
/// environment that has none of what the user's profile puts on `PATH`.
pub const SHELL_LOGIN_ARGS: &[&str] = &["-lc"];

/// The fallback shell's arguments. `-l` is dropped here: `/bin/sh` is `dash`
/// on the Linux CI runner, which does not accept it, and a fallback shell has
/// no profile worth loading anyway.
pub const SHELL_FALLBACK_ARGS: &[&str] = &["-c"];
