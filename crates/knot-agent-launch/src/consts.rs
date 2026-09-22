//! Fixed strings the registration prompts embed. Kept in one place per the
//! workspace's constants convention.

/// Sent on first launch (ACP protocol prompt, or the deferred shell-agent
/// registration prompt) to trigger the agent list table and status set.
///
/// One line, like the instructions it is appended to: the shell-agent path
/// types the combined prompt into a terminal and then sends Return.
pub const REGISTRATION_USER_PROMPT: &str = "List the other agents and their projects (no IDs) in a table, then set your status to ready. Register with the knot if you are not in that table.";

/// Directories a Finder-launched macOS app won't have on its launchd `PATH`
/// (the GUI default is `/usr/bin:/bin:/usr/sbin:/sbin`) but coding-agent
/// CLIs and their ACP adapters are typically installed into. `~`-prefixed
/// entries are resolved against `HOME` at call time - a GUI process keeps
/// `HOME` even though it lost the shell environment.
pub const ADAPTER_PATH_FALLBACK_DIRS: &[&str] = &["/opt/homebrew/bin",
                                                  "/usr/local/bin",
                                                  "~/.cargo/bin",
                                                  "~/.local/bin",
                                                  "~/.npm-global/bin"];
