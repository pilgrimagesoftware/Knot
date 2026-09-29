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

/// The Claude CLI flag that starts a session with every permission check
/// skipped.
pub const CLAUDE_SKIP_PERMISSIONS_FLAG: &str = "dangerously-skip-permissions";

/// The Claude CLI flag naming the permission mode to start in.
pub const CLAUDE_PERMISSION_MODE_FLAG: &str = "permission-mode";

/// The `claude-agent-acp` mode [`CLAUDE_SKIP_PERMISSIONS_FLAG`] starts in.
pub const CLAUDE_BYPASS_MODE: &str = "bypassPermissions";

/// Every `--permission-mode` spelling Claude Code accepts, lowercased, and
/// the `claude-agent-acp` mode id it names. Mirrors the adapter's own alias
/// table, `manual` included.
pub const CLAUDE_PERMISSION_MODES: [(&str, &str); 8] = [("auto", "auto"),
                                                        ("default", "default"),
                                                        ("manual", "default"),
                                                        ("acceptedits", "acceptEdits"),
                                                        ("dontask", "dontAsk"),
                                                        ("plan", "plan"),
                                                        ("bypasspermissions", CLAUDE_BYPASS_MODE),
                                                        ("bypass", CLAUDE_BYPASS_MODE)];

/// Claude CLI flags `claude-agent-acp` passes itself to drive the session
/// over stdio. A second copy from the user would change the wire format or
/// the session under the adapter, so these are not forwarded.
pub const CLAUDE_ADAPTER_FLAGS: [&str; 13] = ["print",
                                              "output-format",
                                              "input-format",
                                              "include-partial-messages",
                                              "replay-user-messages",
                                              "permission-prompt-tool",
                                              "mcp-config",
                                              "setting-sources",
                                              "session-id",
                                              "resume",
                                              "continue",
                                              "fork-session",
                                              "allow-dangerously-skip-permissions"];

/// The environment variable `codex-acp` reads as a JSON object of Codex
/// config overrides and sends on every `thread/start` and `thread/resume`.
pub const CODEX_CONFIG_ENV: &str = "CODEX_CONFIG";

/// The Codex config key whose text becomes a developer message alongside
/// the model's base instructions, rather than replacing them as
/// `base_instructions` would.
pub const CODEX_DEVELOPER_INSTRUCTIONS_KEY: &str = "developer_instructions";

/// The environment variable opencode merges last over its config files, as
/// inline JSON.
pub const OPENCODE_CONFIG_ENV: &str = "OPENCODE_CONFIG_CONTENT";

/// The opencode config key listing instruction files to append to the
/// system prompt. Paths, globs or URLs - not inline text.
pub const OPENCODE_INSTRUCTIONS_KEY: &str = "instructions";

/// Where under the app's cache directory each agent's opencode instructions
/// file is written, one `<agent id>.md` per agent.
pub const INSTRUCTIONS_FILES_DIR: &str = "agent-instructions";
