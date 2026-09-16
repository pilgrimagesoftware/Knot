//! Per-agent-type ACP adapter registry: for each agent type, whether it has
//! a known way to speak ACP (command template + capability flags) or falls
//! back to the terminal launch path unchanged.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`'s "ACP launch
//! path" requirement, design decision 2.

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
}

/// Looks up `agent_type`'s ACP adapter, if any is confirmed working.
///
/// No adapter has been verified against a live agent yet (the research
/// spike in `openspec/changes/acp-agent-panel-ui/tasks.md` task 1.1 was
/// deliberately skipped for this pass), so every agent type currently
/// returns `None` and launches through the existing terminal path per the
/// "Panel-mode agent with no adapter" fallback requirement. Entries are
/// added here one agent type at a time as each adapter is verified working
/// in practice, per the change's rollout plan - a wrong or premature guess
/// here costs one config entry, not a design change.
pub fn acp_adapter(_agent_type: &str) -> Option<AdapterConfig> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_agent_type_has_a_confirmed_adapter_yet() {
        for agent_type in ["claude",
                           "codex",
                           "opencode",
                           "gemini",
                           "copilot",
                           "shell",
                           "custom1",
                           "custom2",
                           "unknown-type"]
        {
            assert_eq!(acp_adapter(agent_type),
                       None,
                       "{agent_type} should have no adapter until one is verified working");
        }
    }
}
