//! Per-agent-type ACP adapter registry: for each agent type, whether it has
//! a known way to speak ACP (command template + capability flags) or falls
//! back to the terminal launch path unchanged.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`'s "ACP launch
//! path" requirement, design decision 2.

use crate::consts::ADAPTER_PATH_FALLBACK_DIRS;

/// The command to run once, on the user's behalf, to make an adapter's
/// binary available when it isn't found on `PATH` - see design.md
/// decision 6. Only declared for adapters with a package-manager install
/// path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallMethod {
    pub command: &'static str,
    pub args:    &'static [&'static str],
}

/// How to launch `agent_type`'s ACP adapter subprocess, and what it
/// supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterConfig {
    /// The adapter binary/command to spawn (e.g. a `*-acp` wrapper, or the
    /// agent's own CLI if it speaks ACP natively).
    pub command:                   &'static str,
    /// Extra fixed arguments the adapter command needs before Knot's own
    /// per-session args (cwd, MCP config).
    pub args:                      &'static [&'static str],
    pub supports_resume:           bool,
    pub supports_permission_modes: bool,
    /// How to install `command` if it isn't found, or `None` if this
    /// adapter type has no declared package-manager install path.
    pub install:                   Option<InstallMethod>,
}

/// Looks up `agent_type`'s ACP adapter, if any is confirmed working.
///
/// Populated from the research spike's findings
/// (`openspec/changes/acp-agent-panel-ui/acp-adapter-findings.md`, task
/// 1.1). `claude` and `gemini` have each been exercised against a live
/// handshake (task 1.2); the rest are per-vendor-doc claims, not yet run.
/// Agent types with no known ACP path (or not otherwise
/// supported by Knot - Cursor, QwenCode) return `None` and launch through
/// the existing terminal path per the "Panel-mode agent with no adapter"
/// fallback requirement. A wrong or premature entry here costs one config
/// fix, not a design change, per the change's rollout plan.
pub fn acp_adapter(agent_type: &str) -> Option<AdapterConfig> {
    match agent_type {
        // Adapter package `@agentclientprotocol/claude-agent-acp` (npm),
        // exposing the `claude-agent-acp` binary once installed - and
        // auto-installed on first use if it isn't (design.md decision 6).
        "claude" => Some(AdapterConfig { command:                   "claude-agent-acp",
                                         args:                      &[],
                                         supports_resume:           true,
                                         supports_permission_modes: true,
                                         install:                   Some(InstallMethod { command: "npm",
                                                                                         args:    &["install",
                                                                                                    "-g",
                                                                                                    "@agentclientprotocol/claude-agent-acp"], }), }),
        // The npm adapter includes a compatible Codex CLI dependency and
        // installs the `codex-acp` executable alongside Claude's adapter.
        // Resume support is undocumented, so this remains disabled.
        "codex" => Some(AdapterConfig { command:                   "codex-acp",
                                        args:                      &[],
                                        supports_resume:           false,
                                        supports_permission_modes: true,
                                        install:                   Some(InstallMethod { command: "npm",
                                                                                        args:    &["install",
                                                                                                   "-g",
                                                                                                   "@agentclientprotocol/codex-acp"], }), }),
        // Native ACP subcommand - no install step (the opencode CLI
        // itself is the agent).
        "opencode" => Some(AdapterConfig { command:                   "opencode",
                                           args:                      &["acp"],
                                           supports_resume:           true,
                                           supports_permission_modes: true,
                                           install:                   None, }),
        // Native ACP flag - no install step.
        // `--skip-trust` is required for headless/automated invocation -
        // confirmed live (task 1.2): without it gemini blocks on an
        // interactive "trust this workspace?" prompt that never resolves
        // when driven over stdio instead of a real terminal.
        "gemini" => Some(AdapterConfig { command:                   "gemini",
                                         args:                      &["--acp", "--skip-trust"],
                                         supports_resume:           true,
                                         supports_permission_modes: true,
                                         install:                   None, }),
        // Native ACP flag (stdio transport, the ACP default) - no install
        // step.
        "copilot" => Some(AdapterConfig { command:                   "copilot",
                                          args:                      &["--acp"],
                                          supports_resume:           true,
                                          supports_permission_modes: true,
                                          install:                   None, }),
        _ => None,
    }
}

/// The `PATH` adapter subprocesses and their install commands are spawned
/// with: the process's own `PATH` first (so a shell-launched app keeps its
/// exact ordering), then the standard install locations from
/// `consts::ADAPTER_PATH_FALLBACK_DIRS` (`~`-expanded against `HOME`), each
/// entry appearing once. A Finder-launched GUI process gains the locations
/// launchd's `/usr/bin:/bin:/usr/sbin:/sbin` omits, so a registered adapter
/// installed in one of them launches instead of failing with "No such file
/// or directory".
pub fn adapter_path() -> String {
    adapter_path_for(&std::env::var("PATH").unwrap_or_default(),
                     &std::env::var("HOME").unwrap_or_default())
}

/// Pure form of [`adapter_path`] for tests and callers already holding the
/// values. `process_path` entries win the ordering; `~`-prefixed fallbacks
/// are expanded against `home` (kept literal when `home` is empty, matching
/// how the child would see an unset `HOME`); an entry already named is not
/// duplicated.
pub fn adapter_path_for(process_path: &str, home: &str) -> String {
    let fallbacks: Vec<String> =
        ADAPTER_PATH_FALLBACK_DIRS.iter()
                                  .map(|dir| match dir.strip_prefix('~') {
                                      Some(suffix) if !home.is_empty() => format!("{home}{suffix}"),
                                      _ => (*dir).to_string(),
                                  })
                                  .collect();

    let mut merged: Vec<&str> = Vec::new();
    for entry in process_path.split(':')
                             .chain(fallbacks.iter().map(String::as_str))
    {
        if entry.is_empty() {
            continue;
        }
        if !merged.contains(&entry) {
            merged.push(entry);
        }
    }
    merged.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmed_agent_types_have_registered_adapters() {
        for (agent_type, command) in [("claude", "claude-agent-acp"),
                                      ("codex", "codex-acp"),
                                      ("opencode", "opencode"),
                                      ("gemini", "gemini"),
                                      ("copilot", "copilot")]
        {
            let adapter = acp_adapter(agent_type);
            assert_eq!(adapter.map(|a| a.command),
                       Some(command),
                       "{agent_type} should have a registered adapter");
        }
    }

    /// The roster is the list of types Knot knows; this table is which of
    /// them speak ACP. A type added there without a decision here would
    /// silently launch through the terminal path, which is a real answer
    /// for `shell` and a bug for a coding agent - so every row has to be
    /// named in one of these two tests.
    #[test]
    fn every_known_agent_type_is_decided_about() {
        let decided =
            ["claude", "codex", "opencode", "gemini", "copilot", "shell", "custom1", "custom2"];
        for kind in knot_core::agent_type::ALL {
            assert!(decided.contains(&kind.id),
                    "{} is in the roster but no adapter test names it",
                    kind.id);
        }
    }

    #[test]
    fn unsupported_or_unknown_agent_types_have_no_adapter() {
        for agent_type in ["shell",
                           "custom1",
                           "custom2",
                           "unknown-type",
                           "cursor",
                           "qwencode"]
        {
            assert_eq!(acp_adapter(agent_type),
                       None,
                       "{agent_type} should have no registered adapter");
        }
    }

    #[test]
    fn codex_resume_is_conservatively_unsupported() {
        // Per the findings note: resume support for the codex-acp adapter
        // is undocumented, so this fails closed rather than guessing.
        assert!(!acp_adapter("codex").unwrap().supports_resume);
    }

    #[test]
    fn claude_and_codex_adapters_can_be_installed() {
        for agent_type in ["claude", "codex"] {
            assert_eq!(acp_adapter(agent_type).unwrap()
                                              .install
                                              .map(|install| install.command),
                       Some("npm"),
                       "{agent_type} should declare its npm installer");
        }
    }

    #[test]
    fn adapter_path_process_entries_come_first() {
        let path = adapter_path_for("/usr/local/bin:/usr/bin", "/Users/tester");
        let entries: Vec<&str> = path.split(':').collect();
        assert_eq!(&entries[..2],
                   &["/usr/local/bin", "/usr/bin"],
                   "the process's own PATH entries must keep their order and come first");
        assert!(entries.contains(&"/opt/homebrew/bin"));
        assert!(entries.contains(&"/Users/tester/.cargo/bin"));
    }

    #[test]
    fn adapter_path_dedups_a_fallback_already_on_the_process_path() {
        let path = adapter_path_for("/opt/homebrew/bin:/usr/bin:/bin", "/Users/tester");
        let entries: Vec<&str> = path.split(':').collect();
        assert_eq!(entries.iter()
                          .filter(|e| **e == "/opt/homebrew/bin")
                          .count(),
                   1,
                   "a fallback already named by the process PATH must not be appended again");
        assert_eq!(entries[0], "/opt/homebrew/bin",
                   "and it keeps the process PATH's ordering (first, in this case)");
    }

    #[test]
    fn adapter_path_expands_tilde_fallbacks_against_home() {
        let path = adapter_path_for("", "/Users/tester");
        let entries: Vec<&str> = path.split(':').collect();
        for expected in ["/Users/tester/.cargo/bin",
                         "/Users/tester/.local/bin",
                         "/Users/tester/.npm-global/bin"]
        {
            assert!(entries.contains(&expected), "missing {expected} in {path}");
        }
        assert!(!entries.iter().any(|e| e.starts_with('~')),
                "no literal `~` entry may survive into the merged PATH");
    }

    #[test]
    fn adapter_path_keeps_literal_tilde_when_home_is_empty() {
        let path = adapter_path_for("", "");
        assert!(path.split(':').any(|e| e == "~/.cargo/bin"),
                "an empty HOME keeps the fallback literal rather than mangling it");
    }

    #[test]
    fn adapter_path_covers_the_launchd_gui_path() {
        // The PATH a Finder-launched macOS app actually sees - the bug
        // scenario this change exists for.
        let path = adapter_path_for("/usr/bin:/bin:/usr/sbin:/sbin", "/Users/tester");
        for expected in ["/opt/homebrew/bin",
                         "/usr/local/bin",
                         "/Users/tester/.cargo/bin",
                         "/Users/tester/.local/bin",
                         "/Users/tester/.npm-global/bin"]
        {
            assert!(path.split(':').any(|e| e == expected),
                    "missing {expected} in {path}");
        }
        assert_eq!(path.split(':').next(),
                   Some("/usr/bin"),
                   "the launchd entries must still come first when present");
    }
}
