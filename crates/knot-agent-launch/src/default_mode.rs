//! The permission mode a panel session starts in when the user has chosen
//! none, for adapters whose own default differs from their CLI's.
//!
//! Contract: `openspec/specs/session-setup-persistence/spec.md`.
//!
//! The Claude CLI starts in Auto mode unless a settings file says otherwise,
//! but `claude-agent-acp` reads only `permissions.defaultMode` and, finding
//! none, starts in Manual - so the same user saw Auto in a terminal and
//! Manual in a Knot panel (#516). Knot applies the CLI's default itself, and
//! only where the CLI would: a `defaultMode` in any settings file Claude
//! reads is the user's choice, and the adapter already honors it.

use std::path::{Path, PathBuf};

use crate::consts::{
    CLAUDE_CONFIG_DIR_DEFAULT, CLAUDE_CONFIG_DIR_ENV, CLAUDE_DEFAULT_MODE,
    CLAUDE_MANAGED_SETTINGS_MACOS, CLAUDE_MODE_CONFIG_ID,
};

/// A session config option to select when nothing else has chosen one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultMode {
    /// The config option's id, as the adapter declares it.
    pub config_id: &'static str,
    /// The value to select.
    pub value:     &'static str,
}

/// `agent_type`'s default mode for a session in `cwd`, or `None` when the
/// adapter's own default is already right - because the type has no
/// differing CLI default, or because a settings file chooses one. Blocks: it
/// reads the settings files.
pub fn unconfigured_default_mode(agent_type: &str, cwd: &Path) -> Option<DefaultMode> {
    default_mode_with(agent_type, cwd, claude_config_dir().as_deref())
}

/// [`unconfigured_default_mode`] with Claude's user config directory given
/// rather than read from the environment.
pub(crate) fn default_mode_with(agent_type: &str, cwd: &Path, config_dir: Option<&Path>)
                                -> Option<DefaultMode> {
    match agent_type {
        "claude" => {
            let files = claude_settings_files(cwd, config_dir);
            (!files.iter().any(|file| sets_default_mode(file))).then_some(DefaultMode {
                config_id: CLAUDE_MODE_CONFIG_ID,
                value:     CLAUDE_DEFAULT_MODE,
            })
        }
        _ => None,
    }
}

/// Claude Code's user config directory: `$CLAUDE_CONFIG_DIR`, else
/// `~/.claude`.
fn claude_config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(CLAUDE_CONFIG_DIR_ENV).filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(CLAUDE_CONFIG_DIR_DEFAULT))
}

/// Every settings file Claude Code reads for a session in `cwd`, whichever
/// exist: managed, project-local, project-shared and user.
pub(crate) fn claude_settings_files(cwd: &Path, config_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from(CLAUDE_MANAGED_SETTINGS_MACOS),
                         cwd.join(".claude/settings.local.json"),
                         cwd.join(".claude/settings.json")];
    files.extend(config_dir.map(|dir| dir.join("settings.json")));
    files
}

/// Whether `file` sets `permissions.defaultMode`. A file that is missing or
/// will not parse sets nothing - Claude ignores it too.
pub(crate) fn sets_default_mode(file: &Path) -> bool {
    std::fs::read_to_string(file).ok()
                                 .and_then(|text| {
                                     serde_json::from_str::<serde_json::Value>(&text).ok()
                                 })
                                 .is_some_and(|settings| {
                                     settings.pointer("/permissions/defaultMode")
                                             .is_some_and(|mode| !mode.is_null())
                                 })
}

#[cfg(test)]
mod tests;
