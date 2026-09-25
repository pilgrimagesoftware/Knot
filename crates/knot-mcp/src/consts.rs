use std::time::Duration;

pub const DEFAULT_PORT: u16 = 8767;
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(3600);
pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "knot-mcp";
pub const SERVER_VERSION: &str = "1.0.0";

/// The active log file's name within whatever directory the caller supplies,
/// one JSON object per line.
/// Rolled files take this name with a `.1`, `.2`, ... suffix.
pub const LOG_FILE_NAME: &str = "knot-mcp.jsonl";

/// Roll the active log file once it reaches this size. Reached in weeks of
/// idle heartbeats, or a long session of heavy tool use - which is the
/// range the cap is chosen for: large enough that a debugging session stays
/// in one file, small enough that the retained set is a few megabytes.
pub const LOG_ROTATION_SIZE: u64 = 5 * 1024 * 1024;

/// How many rolled files to keep beside the active one. Older ones are
/// deleted, so the log occupies at most `(1 + this) * LOG_ROTATION_SIZE`.
pub const LOG_RETAINED_FILES: usize = 3;

/// How often the server writes its vitals while it is serving. A minute is
/// short enough that a gap in the log localizes a death, and long enough that
/// an idle server does not fill the file on its own.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// The wait before the supervisor's second start attempt. Short enough that
/// a port released a moment after launch - the previous Knot process still
/// closing its listener - is picked up without the user noticing.
pub const BACKOFF_INITIAL_DELAY: Duration = Duration::from_millis(500);

/// What each failed attempt multiplies the delay by.
pub const BACKOFF_MULTIPLIER: u32 = 2;

/// The ceiling the delay grows to. Supervision retries forever, so this is
/// the cadence it settles at against a permanently held port: often enough
/// to recover promptly, rare enough not to spin.
pub const BACKOFF_MAX_DELAY: Duration = Duration::from_secs(30);

/// How often a running server is probed for liveness.
pub const PROBE_INTERVAL: Duration = Duration::from_secs(10);

/// The budget for one probe, covering connect, write and read together. A
/// loopback request that has not answered in this long is not slow, it is
/// wedged.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// How many probes in a row must fail before the server is restarted. More
/// than one, so a single dropped connection does not bounce a healthy
/// server.
pub const PROBE_FAILURE_THRESHOLD: u32 = 3;
