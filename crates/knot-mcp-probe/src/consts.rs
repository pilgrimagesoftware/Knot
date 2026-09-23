use std::time::Duration;

/// How long a probe may run before it is killed.
///
/// Longer than `knot-forge`'s bound, and for the same reason it has one at
/// all: `claude mcp list` health-checks every configured server as it goes,
/// so a probe of a dozen HTTP servers is a dozen round trips and one
/// unreachable host can hold the whole listing open. The bound stops a hung
/// child owning the section forever; it is not meant to be tight.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the timeout loop checks whether the child has exited.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// The longest identifier a row will show for a server's target.
///
/// A stdio server's command line is unbounded - one entry observed in the
/// wild was a 1.5 KB `node -e` program - and the row that renders it sits in
/// a virtualized list, where an over-tall row misreports its own height.
pub const MAX_LABEL_CHARS: usize = 48;
