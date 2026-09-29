//! Fixed strings the registration prompts embed. Kept in one place per the
//! workspace's constants convention.

use crate::agent_options::FlagSpec;

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

/// The Codex CLI flags whose effect `codex-acp` lets through, as config
/// overrides in [`CODEX_CONFIG_ENV`]. Its argv never reaches Codex, and it
/// sets approval and sandbox policy itself on every turn from the session's
/// mode, so `-s`, `-a` and the flags built on them are not here.
pub(crate) const CODEX_OPTION_FLAGS: [FlagSpec; 5] = [FlagSpec::value("config", Some('c')),
                                                      FlagSpec::value("model", Some('m')),
                                                      FlagSpec::value("profile", Some('p')),
                                                      FlagSpec::value("enable", None),
                                                      FlagSpec::value("disable", None)];

/// The opencode flags `opencode acp` can honor: `--model` and `--agent` as
/// the `model` and `default_agent` of [`OPENCODE_CONFIG_ENV`], the rest as
/// arguments the `acp` subcommand itself accepts.
pub(crate) const OPENCODE_OPTION_FLAGS: [FlagSpec; 5] = [FlagSpec::value("model", Some('m')),
                                                         FlagSpec::value("agent", None),
                                                         FlagSpec::switch("print-logs", None),
                                                         FlagSpec::value("log-level", None),
                                                         FlagSpec::switch("pure", None)];

/// The Gemini CLI flags forwarded to `gemini --acp`. Left out: the prompt,
/// output, session, worktree and listing flags, which would turn an ACP
/// session into something else, and `--debug`, which opens a console.
pub(crate) const GEMINI_OPTION_FLAGS: [FlagSpec; 12] =
    [FlagSpec::value("model", Some('m')),
     FlagSpec::switch("sandbox", Some('s')),
     FlagSpec::switch("yolo", Some('y')),
     FlagSpec::value("approval-mode", None),
     FlagSpec::value("policy", None),
     FlagSpec::value("admin-policy", None),
     FlagSpec::value("allowed-mcp-server-names", None),
     FlagSpec::value("allowed-tools", None),
     FlagSpec::value("extensions", Some('e')),
     FlagSpec::value("include-directories", None),
     FlagSpec::switch("raw-output", None),
     FlagSpec::switch("accept-raw-output-risk", None)];

/// The opencode config key naming the agent a session starts in.
pub const OPENCODE_DEFAULT_AGENT_KEY: &str = "default_agent";

/// The config key, in both Codex and opencode, naming the model.
pub const MODEL_CONFIG_KEY: &str = "model";

/// The Codex config table `--enable`/`--disable` switch features in.
pub const CODEX_FEATURES_KEY: &str = "features";

/// The Codex config key `--profile` names.
pub const CODEX_PROFILE_KEY: &str = "profile";

/// Sent on a resumed session whose adapter may not keep the agent's MCP URL
/// intact, so the server could not register it from the connection (#552).
/// One line and the request alone: the instructions are already in the
/// system channel, and the conversation has its context.
pub const RESUME_REGISTRATION_PROMPT: &str =
    "Knot restarted. Call register-agent with your knot agent ID; nothing else is needed.";

/// Agent types whose ACP adapter keeps the `?agent=<id>` query on every MCP
/// request - checked live against `claude-agent-acp` 0.82.0 and `codex-acp`
/// (#544) - so their connection registers them and a resume needs no turn.
pub const MCP_QUERY_KEEPING_TYPES: [&str; 2] = ["claude", "codex"];
