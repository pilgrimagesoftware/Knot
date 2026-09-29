//! Where this application keeps files it regenerates on demand.
//!
//! Not [`crate::settings::StorePaths`]: those directories hold the durable
//! settings documents, and nothing here is one. A file in this directory is
//! rewritten by whoever needs it before it is read, so the system deleting
//! it - which a cache directory permits - costs nothing.

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::consts::{APP_NAME, ORG_NAME, ORG_QUALIFIER};

/// The platform cache directory for this application, or `None` when no home
/// directory can be resolved. On macOS, `~/Library/Caches/<bundle-like id>`.
pub fn cache_dir() -> Option<PathBuf> {
    ProjectDirs::from(ORG_QUALIFIER, ORG_NAME, APP_NAME).map(|dirs| dirs.cache_dir().to_path_buf())
}

#[cfg(test)]
mod tests;
