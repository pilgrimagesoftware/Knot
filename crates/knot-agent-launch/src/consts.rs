//! Fixed strings the registration prompts embed. Kept in one place per the
//! workspace's constants convention.

/// Sent on first launch (ACP protocol prompt, or the deferred shell-agent
/// registration prompt) to trigger the agent list table and status set.
///
/// One line, like the instructions it is appended to: the shell-agent path
/// types the combined prompt into a terminal and then sends Return.
pub const REGISTRATION_USER_PROMPT: &str = "List the other agents and their projects (no IDs) in a table, then set your status to ready. Register with the knot if you are not in that table.";

/// The ACP session config option `claude-agent-acp` names its permission
/// mode by.
pub const CLAUDE_MODE_CONFIG_ID: &str = "mode";

/// The permission mode the Claude CLI starts in when nothing configures one.
pub const CLAUDE_DEFAULT_MODE: &str = "auto";

/// Where Claude Code keeps its user settings, relative to the home directory,
/// unless `CLAUDE_CONFIG_DIR` names another directory.
pub const CLAUDE_CONFIG_DIR_DEFAULT: &str = ".claude";

/// The environment variable that moves Claude Code's user config directory.
pub const CLAUDE_CONFIG_DIR_ENV: &str = "CLAUDE_CONFIG_DIR";

/// Claude Code's system-wide managed settings on macOS.
pub const CLAUDE_MANAGED_SETTINGS_MACOS: &str =
    "/Library/Application Support/ClaudeCode/managed-settings.json";
