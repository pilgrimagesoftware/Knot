//! Per-agent-type capability lookups the activity tracker consults to
//! decide between inline and deferred registration.

/// Whether `agent_type` supports inline registration via CLI arguments.
///
/// Read from `knot_core::agent_type`'s roster rather than listed again
/// here: `shell` carries the flag so shell agents skip the deferred
/// registration path entirely, and a type the build does not know does not,
/// so it takes the deferred path like any other unknown.
pub fn supports_inline_registration(agent_type: &str) -> bool {
    knot_core::agent_type::supports_inline_registration(agent_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_registration_support_matches_spec_table() {
        for t in ["claude", "codex", "opencode", "gemini", "copilot", "shell"] {
            assert!(supports_inline_registration(t),
                    "{t} should support inline registration");
        }
        assert!(!supports_inline_registration("unknown-type"));
    }
}
