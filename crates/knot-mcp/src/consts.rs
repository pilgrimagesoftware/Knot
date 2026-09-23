use std::time::Duration;

pub const DEFAULT_PORT: u16 = 8767;
pub const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(3600);
pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "knot-mcp";
pub const SERVER_VERSION: &str = "1.0.0";

/// The active log file's name within whatever directory the caller supplies.
/// Rolled files take this name with a `.1`, `.2`, ... suffix.
pub const LOG_FILE_NAME: &str = "knot-mcp.log";

/// Roll the active log file once it reaches this size. Reached in weeks of
/// idle heartbeats, or a long session of heavy tool use - which is the
/// range the cap is chosen for: large enough that a debugging session stays
/// in one file, small enough that the retained set is a few megabytes.
pub const LOG_ROTATION_SIZE: u64 = 5 * 1024 * 1024;

/// How many rolled files to keep beside the active one. Older ones are
/// deleted, so the log occupies at most `(1 + this) * LOG_ROTATION_SIZE`.
pub const LOG_RETAINED_FILES: usize = 3;
