//! Fixed strings the registration prompts embed. Kept in one place per the
//! workspace's constants convention.

/// Sent on first launch (ACP protocol prompt, or the deferred shell-agent
/// registration prompt) to trigger the agent list table and status set.
pub const REGISTRATION_USER_PROMPT: &str = "List other agents names and project (no ID) in a table based on context then set your status to indicate you are ready to get going. If you don't see yourself in the table, register with the knot.";

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
