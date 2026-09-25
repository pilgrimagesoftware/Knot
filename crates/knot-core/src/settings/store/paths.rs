//! Where each settings document lives.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! The store is three kinds of document in two directories: the preferences
//! document in the platform's user-preferences directory, and - in the
//! application-data directory - one document per durable collection plus the
//! per-workspace UI-state document. The third kind is distinguished by what
//! its values are, not by where it lives: UI state is what the application
//! recorded about its own windows, which the user never entered.
//! [`StorePaths`] holds the two locations and derives every document's path
//! from them, so nothing else has to know a filename.

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::consts::{
    AGENTS_FILE, APP_NAME, BENCH_FILE, LEGACY_SETTINGS_FILE, ORG_NAME, ORG_QUALIFIER,
    PERSONAS_FILE, PREFERENCES_FILE, PROMPTS_FILE, PULL_REQUESTS_FILE, RECENT_REPOS_FILE,
    WORKSPACE_UI_STATE_FILE, WORKSPACES_FILE,
};

/// The resolved location of every document the store reads and writes.
///
/// On macOS the two directories differ (`~/Library/Preferences` and
/// `~/Library/Application Support`); on Linux and Windows they are the same
/// directory, and the documents stay distinct by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    preferences_dir: PathBuf,
    data_dir:        PathBuf,
}

impl StorePaths {
    /// The platform's directories for this application, or `None` when no
    /// home directory could be resolved.
    pub fn platform() -> Option<Self> {
        ProjectDirs::from(ORG_QUALIFIER, ORG_NAME, APP_NAME).map(|dirs| {
                                                                Self { preferences_dir:
                                                                           dirs.preference_dir()
                                                                               .to_path_buf(),
                                                                       data_dir:
                                                                           dirs.config_dir()
                                                                               .to_path_buf(), }
                                                            })
    }

    /// Both kinds of document under one directory. Tests use this; it is also
    /// what an alternate profile would be rooted at.
    pub fn rooted(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self { preferences_dir: dir.clone(),
               data_dir:        dir, }
    }

    /// The preferences document, holding every scalar setting.
    pub fn preferences(&self) -> PathBuf {
        self.preferences_dir.join(PREFERENCES_FILE)
    }

    /// The saved-agents document.
    pub fn agents(&self) -> PathBuf {
        self.data_dir.join(AGENTS_FILE)
    }

    /// The saved-workspaces document.
    pub fn workspaces(&self) -> PathBuf {
        self.data_dir.join(WORKSPACES_FILE)
    }

    /// The per-workspace UI-state document.
    pub fn workspace_ui_state(&self) -> PathBuf {
        self.data_dir.join(WORKSPACE_UI_STATE_FILE)
    }

    /// The personas document.
    pub fn personas(&self) -> PathBuf {
        self.data_dir.join(PERSONAS_FILE)
    }

    /// The bench-templates document.
    pub fn bench(&self) -> PathBuf {
        self.data_dir.join(BENCH_FILE)
    }

    /// The prompt-library document.
    pub fn prompts(&self) -> PathBuf {
        self.data_dir.join(PROMPTS_FILE)
    }

    /// The recent-repositories document.
    pub fn recent_repos(&self) -> PathBuf {
        self.data_dir.join(RECENT_REPOS_FILE)
    }

    /// The recorded-pull-requests document.
    pub fn pull_requests(&self) -> PathBuf {
        self.data_dir.join(PULL_REQUESTS_FILE)
    }

    /// The single document the store used to be, read once and renamed by the
    /// migration in [`super::legacy`].
    pub fn legacy(&self) -> PathBuf {
        self.data_dir.join(LEGACY_SETTINGS_FILE)
    }
}

#[cfg(test)]
mod tests;
