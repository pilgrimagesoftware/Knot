//! Per-agent-type ACP adapter registry: for each agent type, whether it has
//! a known way to speak ACP (command template + capability flags) or falls
//! back to the terminal launch path unchanged.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`'s "ACP launch
//! path" requirement, design decision 2.

/// The command to run once, on the user's behalf, to make an adapter's
/// binary available when it isn't found on `PATH` - see design.md
/// decision 6. Only declared for adapters with a package-manager install
/// path; an adapter that needs a source build (e.g. `codex-acp`) has none
/// and fails closed with a visible error instead, per the same decision.
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
        // Adapter `cola-io/codex-acp`, built from source (no packaged
        // binary release confirmed, so no declared install method - a
        // missing binary here fails closed rather than attempting a
        // source build). Resume support is undocumented, so this fails
        // closed (false) rather than guessing.
        "codex" => Some(AdapterConfig { command:                   "codex-acp",
                                        args:                      &[],
                                        supports_resume:           false,
                                        supports_permission_modes: true,
                                        install:                   None, }),
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
}
