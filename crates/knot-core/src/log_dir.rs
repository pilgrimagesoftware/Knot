//! Where this application's log files live.
//!
//! Deliberately not part of [`crate::settings::StorePaths`]. That type's two
//! directories are where settings *documents* go, and its contract
//! (`openspec/specs/settings-persistence/spec.md`) describes the
//! application-data directory as holding one document per durable
//! collection. A log is not one of those, and on macOS
//! `~/Library/Application Support` is the wrong place for it regardless -
//! the platform has a directory for logs, and it is the one Console.app
//! reads.

use std::path::PathBuf;

use directories::{BaseDirs, ProjectDirs};

use crate::consts::{APP_NAME, ORG_NAME, ORG_QUALIFIER};

/// The directory this application writes logs into, or `None` when no home
/// directory can be resolved.
///
/// On macOS this is `~/Library/Logs/<app>`, which is where the platform
/// expects an application's logs and where Console.app looks for them.
/// `ProjectDirs` offers no logs directory, so that path is composed from the
/// home directory directly.
///
/// Elsewhere it is the platform's state directory when there is one (on
/// Linux, `~/.local/state/<app>`), falling back to the data directory. A
/// cache directory would be wrong on every platform: logs are wanted
/// precisely when something went wrong, and a cache is what the system
/// deletes first.
pub fn log_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        return BaseDirs::new().map(|dirs| dirs.home_dir().join("Library/Logs").join(APP_NAME));
    }
    let dirs = ProjectDirs::from(ORG_QUALIFIER, ORG_NAME, APP_NAME)?;
    Some(dirs.state_dir()
             .unwrap_or_else(|| dirs.data_dir())
             .to_path_buf())
}

#[cfg(test)]
mod tests;
