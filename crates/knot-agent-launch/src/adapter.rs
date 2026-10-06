//! Per-agent-type ACP adapter registry: for each agent type, whether it has
//! a known way to speak ACP (command template + capability flags) or falls
//! back to the terminal launch path unchanged.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`'s "ACP launch
//! path" requirement, design decision 2.

/// The command to run once, on the user's behalf, to make an adapter's
/// binary available when it isn't found on `PATH` - see design.md
/// decision 6. Only declared for adapters with a package-manager install
/// path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstallMethod {
    pub command: &'static str,
    pub args:    &'static [&'static str],
    /// The installed package's name, for an update check (`npm view
    /// <package> version` vs. the locally installed version) - kept
    /// separate from `args` so a check doesn't have to parse the install
    /// command back apart to find it.
    pub package: &'static str,
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
    /// Whether Knot should try to resume this type's prior session.
    ///
    /// A claim about the adapter family, reported to other agents as the
    /// `resume` tag. It is not what decides a resume: a connected adapter
    /// is asked for `session/load` only if its `initialize` advertises
    /// `loadSession`, so a version that predates loading stays fresh.
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
                                                                                                    "@agentclientprotocol/claude-agent-acp"],
                                                                                         package: "@agentclientprotocol/claude-agent-acp", }), }),
        // The npm adapter includes a compatible Codex CLI dependency and
        // installs the `codex-acp` executable alongside Claude's adapter.
        // `codex-acp` 2.0.0 implements `session/load` and advertises
        // `loadSession` (#540); an older one that does not is never asked,
        // since the resume is gated on what the adapter advertises.
        "codex" => Some(AdapterConfig { command:                   "codex-acp",
                                        args:                      &[],
                                        supports_resume:           true,
                                        supports_permission_modes: true,
                                        install:                   Some(InstallMethod { command: "npm",
                                                                                        args:    &["install",
                                                                                                   "-g",
                                                                                                   "@agentclientprotocol/codex-acp"],
                                                                                        package: "@agentclientprotocol/codex-acp", }), }),
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
/// with: [`knot_core::exec_path::search_path`], which is the process's own
/// `PATH` followed by the standard install locations. A Finder-launched GUI
/// process gains the locations launchd's `/usr/bin:/bin:/usr/sbin:/sbin`
/// omits, so a registered adapter installed in one of them launches instead
/// of failing with "No such file or directory".
///
/// Thin wrapper rather than a re-export: this crate's callers ask for "the
/// path an adapter runs under", and the answer happening to be the shared
/// one is not something they should have to know.
#[must_use]
pub fn adapter_path() -> String {
    knot_core::exec_path::search_path()
}

/// Pure form of [`adapter_path`] for tests and callers already holding the
/// values.
#[must_use]
pub fn adapter_path_for(process_path: &str, home: &str) -> String {
    knot_core::exec_path::search_path_for(process_path, home)
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

    /// `codex-acp` 2.0.0 loads sessions (#540). Whether a given install
    /// does is its advertised `loadSession`, checked at connect; this is
    /// only what Knot tries and reports.
    #[test]
    fn codex_resume_is_supported() {
        assert!(acp_adapter("codex").unwrap().supports_resume);
    }

    /// Every registered adapter is now resume-capable, so the `resume` tag
    /// no longer tells types apart - but a new adapter added without it
    /// should be a decision, not an accident.
    #[test]
    fn every_registered_adapter_supports_resume() {
        for agent_type in ["claude", "codex", "opencode", "gemini", "copilot"] {
            assert!(acp_adapter(agent_type).unwrap().supports_resume,
                    "{agent_type} should try to resume");
        }
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

    /// The update check (`npm view <package> version`) names the package
    /// directly rather than re-deriving it from `args`, so this is the one
    /// place that pairing can drift - a wrong `package` would check a
    /// different package's version than the one `args` actually installs.
    #[test]
    fn declared_package_matches_the_install_args() {
        for agent_type in ["claude", "codex"] {
            let install = acp_adapter(agent_type).unwrap().install.unwrap();
            assert_eq!(install.args.last(),
                       Some(&install.package),
                       "{agent_type}'s declared package should match its install args");
        }
    }

    /// The merging itself is `knot_core::exec_path`'s, and tested there.
    /// This is the capability's own guarantee: the path an adapter is
    /// spawned under names the standard install locations even when the app
    /// was launched from Finder.
    #[test]
    fn adapter_path_covers_the_launchd_gui_path() {
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
