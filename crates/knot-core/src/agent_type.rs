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
//! launch, whether its hooks drive an activity tracker, how it reports the
//! subagents it dispatches - is a column here,
//! and the pickers are built by filtering this roster rather than by
//! repeating it. Adding a type is one row, plus whatever genuinely needs
//! per-type data of its own (an ACP adapter in `knot-agent-launch`, an icon
//! and an MCP install command in `knot`, a transcript reader in
//! `knot-history`); each of those has a test that fails when a row here has
//! nothing matching it.

use crate::ViewMode;

/// How an agent type reports the subagents it dispatches, if it reports them
/// at all.
///
/// Contract: `openspec/specs/agent-subagents/spec.md` - "An agent type states
/// whether it can report subagents".
///
/// Closed, with no default, because the whole point of the column is that
/// [`None`] and "has dispatched none" are opposite answers. A default would
/// collapse them and make the processes section tell a user that an agent it
/// cannot see into dispatched nothing.
///
/// Resolved against the agent's view mode rather than read directly: a type
/// can speak its protocol in Panel mode and post hooks in Terminal mode, and
/// those are different answers for the same row. See [`Self::can_report`].
///
/// [`None`]: Self::None
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubagentReporting {
    /// Nothing to read. A shell agent, or a type whose adapter has not been
    /// examined yet - the processes section says it cannot tell.
    None,
    /// Through the tool calls of an ACP session, so only in Panel mode.
    ToolCalls,
    /// Through hook events posted to Knot's status route, so only in Terminal
    /// mode.
    Hooks,
    /// Both, each in the view mode that carries it.
    Either,
}

impl SubagentReporting {
    /// Whether an agent of this type, running in `view_mode`, can report its
    /// subagents at all.
    ///
    /// The exhaustive match is the point: a fifth variant fails to compile
    /// here rather than falling through to `false`, which would read on screen
    /// as an agent that dispatched nothing.
    #[must_use]
    pub const fn can_report(self, view_mode: ViewMode) -> bool {
        match (self, view_mode) {
            (Self::None, _) => false,
            (Self::Either, _) => true,
            (Self::ToolCalls, ViewMode::Panel) => true,
            (Self::ToolCalls, ViewMode::Terminal) => false,
            (Self::Hooks, ViewMode::Terminal) => true,
            (Self::Hooks, ViewMode::Panel) => false,
        }
    }
}

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
    /// How this type reports the subagents it dispatches, per view mode.
    ///
    /// `claude` is `ToolCalls` rather than `Either` on purpose. The hook
    /// emitter is a plugin outside this repo, so claiming `Hooks` would make
    /// a Terminal-mode agent report that it dispatched nothing when the
    /// truth is that nothing is sending the events. Flipping this to
    /// `Either` is the one-line follow-up once that plugin ships.
    pub subagents:           SubagentReporting,
}

/// Every known type, in the order a picker offers them.
pub const ALL: &[AgentTypeInfo] = &[AgentTypeInfo { id:                  "claude",
                                                    label:               "Claude",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       true,
                                                    subagents:
                                                        SubagentReporting::ToolCalls, },
                                    AgentTypeInfo { id:                  "codex",
                                                    label:               "Codex",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       true,
                                                    subagents:           SubagentReporting::None, },
                                    AgentTypeInfo { id:                  "opencode",
                                                    label:               "OpenCode",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false,
                                                    subagents:           SubagentReporting::None, },
                                    AgentTypeInfo { id:                  "gemini",
                                                    label:               "Gemini",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false,
                                                    subagents:           SubagentReporting::None, },
                                    AgentTypeInfo { id:                  "copilot",
                                                    label:               "Copilot",
                                                    is_shell:            false,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false,
                                                    subagents:           SubagentReporting::None, },
                                    AgentTypeInfo { id:                  "custom1",
                                                    label:               "Custom 1",
                                                    is_shell:            false,
                                                    is_custom:           true,
                                                    inline_registration: false,
                                                    hook_activity:       false,
                                                    subagents:           SubagentReporting::None, },
                                    AgentTypeInfo { id:                  "custom2",
                                                    label:               "Custom 2",
                                                    is_shell:            false,
                                                    is_custom:           true,
                                                    inline_registration: false,
                                                    hook_activity:       false,
                                                    subagents:           SubagentReporting::None, },
                                    AgentTypeInfo { id:                  "shell",
                                                    label:               "Shell",
                                                    is_shell:            true,
                                                    is_custom:           false,
                                                    inline_registration: true,
                                                    hook_activity:       false,
                                                    subagents:           SubagentReporting::None, }];

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

/// How `id` reports its subagents, or [`SubagentReporting::None`] for a type
/// this build does not recognize - which is the honest answer, since an
/// unrecognized type has no recognizer either.
#[must_use]
pub fn subagent_reporting(id: &str) -> SubagentReporting {
    info(id).map_or(SubagentReporting::None, |agent_type| agent_type.subagents)
}

/// Whether an agent of type `id`, running in `view_mode`, can report its
/// subagents. The question the processes section asks before deciding whether
/// to show a subagents group at all.
#[must_use]
pub fn reports_subagents(id: &str, view_mode: ViewMode) -> bool {
    subagent_reporting(id).can_report(view_mode)
}

/// Whether `id`'s hooks can drive an activity tracker.
#[must_use]
pub fn has_hook_activity(id: &str) -> bool {
    info(id).is_some_and(|agent_type| agent_type.hook_activity)
}

#[cfg(test)]
mod tests;
