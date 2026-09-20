//! Per-agent-type capability lookups the activity tracker consults to
//! decide between inline and deferred registration.

/// Whether `agent_type` supports inline registration via CLI arguments.
/// `shell` is included so shell agents skip the deferred registration path
/// entirely.
pub fn supports_inline_registration(agent_type: &str) -> bool {
    matches!(
        agent_type,
        "claude" | "codex" | "opencode" | "gemini" | "copilot" | "shell"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_registration_support_matches_spec_table() {
        for t in ["claude", "codex", "opencode", "gemini", "copilot", "shell"] {
            assert!(
                supports_inline_registration(t),
                "{t} should support inline registration"
            );
        }
        assert!(!supports_inline_registration("unknown-type"));
    }
}
