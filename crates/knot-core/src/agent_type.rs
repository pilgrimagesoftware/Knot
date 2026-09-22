//! The agent types Knot knows about, and what each one implies.
//!
//! `agent_type` is a `String` on purpose: the MCP `agent_create` tool takes
//! whatever an agent asks for, and `knot-agent-launch` falls back to the
//! terminal path for a type it has no adapter for, so an unrecognized value
//! is a working configuration rather than corrupt data. That is the
//! difference between this and the closed vocabularies in
//! [`crate::settings::vocabulary`], which are enums.
//!
//! What it is *not* is a reason to spell the same list out in six places.
//! Every decision that depends only on which known type this is - its
//! label, whether it is a bare shell, whether it registers itself at
//! launch, whether its hooks drive an activity tracker - is a column here,
//! and the pickers are built by filtering this roster rather than by
//! repeating it. Adding a type is one row, plus whatever genuinely needs
//! per-type data of its own (an ACP adapter in `knot-agent-launch`, an icon
//! and an MCP install command in `knot`, a transcript reader in
//! `knot-history`); each of those has a test that fails when a row here has
//! nothing matching it.

/// One known agent type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentTypeInfo {
    /// The value stored in `Agent::agent_type` and accepted over MCP.
    pub id:                  &'static str,
    /// The product's own name, shown in pickers. Not localized: these are
    /// names, the same in every language.
    pub label:               &'static str,
    /// A bare shell with no AI: no ACP adapter, no activity tracking, and
    /// the only type that runs in `ViewMode::Terminal`.
    pub is_shell:            bool,
    /// A user-configured command rather than a vendor's agent. Offered when
    /// creating an agent, but not where the choice implies vendor-specific
    /// support (an MCP install command, a default for new agents).
    pub is_custom:           bool,
    /// Registers itself with Knot's MCP server through CLI arguments at
    /// launch, rather than needing a prompt afterwards.
    pub inline_registration: bool,
    /// Reports progress through hooks Knot can turn into an activity
    /// tracker, so its status updates without polling.
    pub hook_activity:       bool,
}

/// Every known type, in the order a picker offers them.
pub const ALL: &[AgentTypeInfo] = &[AgentTypeInfo { id:                  "claude",
                                                    label:               "Claude",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       true, },
                                    AgentTypeInfo { id:                  "codex",
                                                    label:               "Codex",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       true, },
                                    AgentTypeInfo { id:                  "opencode",
                                                    label:               "OpenCode",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false, },
                                    AgentTypeInfo { id:                  "gemini",
                                                    label:               "Gemini",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false, },
                                    AgentTypeInfo { id:                  "copilot",
                                                    label:               "Copilot",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false, },
                                    AgentTypeInfo { id:                  "custom1",
                                                    label:               "Custom 1",
                                                    is_shell:            false,
                                                    is_custom:           true,
                                                    inline_registration: false,
                                                    hook_activity:       false, },
                                    AgentTypeInfo { id:                  "custom2",
                                                    label:               "Custom 2",
                                                    is_shell:            false,
                                                    is_custom:           true,
                                                    inline_registration: false,
                                                    hook_activity:       false, },
                                    AgentTypeInfo { id:                  "shell",
                                                    label:               "Shell",
                                                    is_shell:            true,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false, }];

/// The type a new agent gets when nothing else says otherwise.
pub const DEFAULT: &str = "claude";

/// The bare-shell type, for the paths that create one deliberately -
/// a shell companion, or the editor stating what a companion must be.
pub const SHELL: &str = "shell";

/// What is known about `id`, or `None` for a type this build does not
/// recognize - which is a working agent that launches through the terminal
/// path, not an error.
#[must_use]
pub fn info(id: &str) -> Option<&'static AgentTypeInfo> {
    ALL.iter().find(|agent_type| agent_type.id == id)
}

/// The name to show for `id`, falling back to the id itself so an
/// unrecognized type is visible as what it is rather than silently drawn as
/// the default.
#[must_use]
pub fn label(id: &str) -> &str {
    info(id).map_or(id, |agent_type| agent_type.label)
}

/// Whether `id` is a bare shell. The one question asked from every crate:
/// it decides the view mode, the launch path, what is tracked, and whether
/// an agent may be a companion.
#[must_use]
pub fn is_shell(id: &str) -> bool {
    info(id).is_some_and(|agent_type| agent_type.is_shell)
}

/// Whether `id` registers itself with the MCP server at launch.
#[must_use]
pub fn supports_inline_registration(id: &str) -> bool {
    info(id).is_some_and(|agent_type| agent_type.inline_registration)
}

/// Whether `id`'s hooks can drive an activity tracker.
#[must_use]
pub fn has_hook_activity(id: &str) -> bool {
    info(id).is_some_and(|agent_type| agent_type.hook_activity)
}

#[cfg(test)]
mod tests;
