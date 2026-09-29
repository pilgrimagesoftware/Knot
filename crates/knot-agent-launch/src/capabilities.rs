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

/// Whether `agent_type`'s adapter keeps its MCP URL's `?agent=` query, so
/// the server registers the agent from its connection. An unlisted type is
/// not assumed to: it is sent a registration turn on resume instead.
pub fn keeps_mcp_query(agent_type: &str) -> bool {
    crate::consts::MCP_QUERY_KEEPING_TYPES.contains(&agent_type)
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

    #[test]
    fn only_checked_adapters_are_trusted_to_keep_the_query() {
        assert!(keeps_mcp_query("claude"));
        assert!(keeps_mcp_query("codex"));
        for t in ["opencode", "gemini", "copilot", "unknown-type"] {
            assert!(!keeps_mcp_query(t), "{t}");
        }
    }
}
