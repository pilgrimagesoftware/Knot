//! The agent-row context menu's model: which entries exist, their labels,
//! and which apply to a given agent.
//!
//! Pure and GPUI-free, so the item set is unit-testable on its own. The menu
//! bar's Agents menu is built from the same list (`agent_menu`), which is
//! what keeps the two menus in the same order and grouping; the wiring that
//! reads an agent's facts and runs an entry is
//! `workspace_window::menus::agent_row`.

/// One entry in the agent-row context menu. An enum rather than a label,
/// so the renderer attaches each handler by matching a variant instead of
/// a string, and the item set stays unit-testable independent of GPUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentMenuEntry {
    Separator,
    NewCompanion,
    NewShellCompanion,
    EditAgent,
    ForkAgent,
    DuplicateAgent,
    MoveToWorkspace,
    SaveToBench,
    BenchAgent,
    OpenIn,
    MarkdownFiles,
    AgentInfo,
    RegisterAgent,
    Deactivate,
    RestartAgent,
    RestartWithNewConversation,
    RemoveAgent,
}

impl AgentMenuEntry {
    // Exhaustiveness fixture: `tests::agent_context_menu` walks this to
    // prove every variant has a label and an ordering.
    #[allow(dead_code)]
    /// Every variant, in the order they appear in a full menu.
    ///
    /// The menu bar's Agents menu pairs each labelled entry with an action,
    /// and `tests::every_agent_menu_entry_is_in_all` walks this list against
    /// an exhaustive match, so a variant added here without an action - or
    /// added to the enum without reaching this list - fails the build.
    pub(crate) const ALL: [Self; 17] = [Self::Separator,
                                        Self::NewCompanion,
                                        Self::NewShellCompanion,
                                        Self::EditAgent,
                                        Self::ForkAgent,
                                        Self::DuplicateAgent,
                                        Self::MoveToWorkspace,
                                        Self::SaveToBench,
                                        Self::BenchAgent,
                                        Self::OpenIn,
                                        Self::MarkdownFiles,
                                        Self::AgentInfo,
                                        Self::RegisterAgent,
                                        Self::Deactivate,
                                        Self::RestartAgent,
                                        Self::RestartWithNewConversation,
                                        Self::RemoveAgent];

    /// The user-visible label, or `None` for a separator. Matches the Swift
    /// reference's strings (`Skwad/Views/Components/AgentContextMenu.swift`),
    /// except that the port keeps "Remove Agent" where the reference says
    /// "Close Agent" - `agent-list-ui` already specifies the former.
    pub(crate) fn label(self) -> Option<String> {
        match self {
            Self::Separator => None,
            Self::NewCompanion => Some(knot_core::l10n::t("menu.agent.new_companion")),
            Self::NewShellCompanion => Some(knot_core::l10n::t("menu.agent.new_shell_companion")),
            Self::EditAgent => Some(knot_core::l10n::t("menu.agent.edit_agent")),
            Self::ForkAgent => Some(knot_core::l10n::t("menu.agent.fork_agent")),
            Self::DuplicateAgent => Some(knot_core::l10n::t("menu.agent.duplicate_agent")),
            Self::MoveToWorkspace => Some(knot_core::l10n::t("menu.agent.move_to_workspace")),
            Self::SaveToBench => Some(knot_core::l10n::t("menu.agent.save_to_bench")),
            Self::BenchAgent => Some(knot_core::l10n::t("menu.agent.bench_agent")),
            Self::OpenIn => Some(knot_core::l10n::t("menu.agent.open_in")),
            Self::MarkdownFiles => Some(knot_core::l10n::t("menu.agent.markdown_files")),
            Self::AgentInfo => Some(knot_core::l10n::t("menu.agent.agent_info")),
            Self::RegisterAgent => Some(knot_core::l10n::t("menu.agent.register_agent")),
            Self::Deactivate => Some(knot_core::l10n::t("menu.agent.deactivate")),
            Self::RestartAgent => Some(knot_core::l10n::t("menu.agent.restart_agent")),
            Self::RestartWithNewConversation => {
                Some(knot_core::l10n::t("menu.agent.restart_new_conversation"))
            }
            Self::RemoveAgent => Some(knot_core::l10n::t("menu.agent.remove_agent")),
        }
    }
}

/// What the context menu needs to know about the row it was opened on.
/// Everything here is read when the menu opens, not when the row renders,
/// so the item set reflects the store's current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct AgentMenuFacts {
    /// A companion can't own companions, be forked, duplicated, moved or
    /// restarted independently of its owner (`agent-lifecycle`).
    pub(crate) is_companion:         bool,
    /// A shell agent has no coding agent to register with MCP.
    pub(crate) is_shell:             bool,
    /// Whether any attached workspace other than this agent's own exists.
    pub(crate) has_move_targets:     bool,
    /// Whether the agent has ever shown a markdown file.
    pub(crate) has_markdown_history: bool,
    /// Whether the agent is running, i.e. has a session to stop. Deactivate
    /// is absent rather than disabled when it is not, matching how this
    /// menu hides every other item that does not apply.
    pub(crate) is_running:           bool,
    /// Whether the agent runs in Panel mode, i.e. over ACP. Only an ACP
    /// session has an `initialize` response naming the agent and its
    /// version, so Agent Info is absent for a Terminal-mode agent.
    pub(crate) is_panel_mode:        bool,
}

impl AgentMenuFacts {
    /// The facts of an agent every item applies to.
    ///
    /// The menu bar's Agents menu is built from these, because it shows the
    /// full item set and disables what the selection cannot do rather than
    /// omitting it (`app-menu`). Reading the shape from
    /// [`agent_context_menu_entries`] rather than listing it again is what
    /// keeps the two menus in the same order and grouping.
    pub(crate) const EVERY_ITEM: Self = Self { is_companion:         false,
                                               is_shell:             false,
                                               has_move_targets:     true,
                                               has_markdown_history: true,
                                               is_running:           true,
                                               is_panel_mode:        true, };
}

/// The agent-row context menu's entries, in order, with dividers.
///
/// Dividers are emitted between non-empty groups only: a row whose whole
/// group is hidden must not leave a doubled or leading separator behind,
/// which is the failure mode of building this list with unconditional
/// `separator()` calls.
pub(crate) fn agent_context_menu_entries(facts: AgentMenuFacts) -> Vec<AgentMenuEntry> {
    use AgentMenuEntry::*;

    let owner_only = !facts.is_companion;
    let groups =
        [vec![NewCompanion, NewShellCompanion].into_iter()
                                              .filter(|_| owner_only)
                                              .collect::<Vec<_>>(),
         [EditAgent].into_iter()
                    .chain([ForkAgent, DuplicateAgent].into_iter()
                                                      .filter(|_| owner_only))
                    .collect(),
         [MoveToWorkspace].into_iter()
                          .filter(|_| owner_only && facts.has_move_targets)
                          .chain([SaveToBench, BenchAgent].into_iter().filter(|_| owner_only))
                          .collect(),
         [OpenIn].into_iter()
                 .chain([MarkdownFiles].into_iter()
                                       .filter(|_| facts.has_markdown_history))
                 .collect(),
         // Deactivate sits with the other session actions, and
         // immediately above Restart Agent: both act on the
         // session rather than on the agent, and Deactivate is
         // the reversible one of the pair. Restart with New
         // Conversation follows Restart Agent, whose other half it
         // is; a shell has no conversation, so for one it would
         // only repeat Restart Agent.
         //
         // Agent Info heads the session group: it reports on the
         // session the actions below it act on.
         [AgentInfo].into_iter()
                    .filter(|_| facts.is_panel_mode)
                    .chain([RegisterAgent].into_iter().filter(|_| !facts.is_shell))
                    .chain([Deactivate].into_iter().filter(|_| facts.is_running))
                    .chain([RestartAgent].into_iter().filter(|_| owner_only))
                    .chain([RestartWithNewConversation].into_iter()
                                                       .filter(|_| owner_only && !facts.is_shell))
                    .chain([RemoveAgent])
                    .collect()];

    let mut entries = Vec::new();
    for group in groups.into_iter().filter(|group| !group.is_empty()) {
        if !entries.is_empty() {
            entries.push(Separator);
        }
        entries.extend(group);
    }
    entries
}
