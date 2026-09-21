use super::*;
// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct WorkspaceRow {
    pub(crate) id:       Uuid,
    pub(crate) name:     String,
    pub(crate) selected: bool,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct AgentRow {
    pub(crate) id:           Uuid,
    pub(crate) avatar:       String,
    pub(crate) name:         String,
    pub(crate) agent_type:   String,
    pub(crate) folder:       String,
    pub(crate) selected:     bool,
    pub(crate) attached:     bool,
    pub(crate) state:        knot_agents::AgentState,
    pub(crate) unread_count: usize,
}

/// User-facing label for the agent's automatic state-machine state, matching
/// the Swift reference's raw strings (not the Rust enum names).
pub(crate) fn state_label(state: knot_agents::AgentState) -> &'static str {
    match state {
        knot_agents::AgentState::Idle => "Idle",
        knot_agents::AgentState::Running => "Working",
        knot_agents::AgentState::Input => "Awaiting input",
        knot_agents::AgentState::Error => "Error",
    }
}

/// Status-dot color for the agent's automatic state. Diverges from the
/// Swift reference (which uses red for both input and error) by giving
/// "awaiting input" its own blue, since it isn't a failure state.
/// Diff-stat colors, drawn from the same palette as [`state_color`]:
/// additions green, deletions red, the changed-file count blue. Only the
/// numbers take these - the words around them stay muted, so the figures
/// are what the eye lands on.
pub(crate) const DIFF_ADDED_COLOR: u32 = consts::COLOR_IDLE;
pub(crate) const DIFF_REMOVED_COLOR: u32 = consts::COLOR_ERROR;
pub(crate) const DIFF_FILES_COLOR: u32 = consts::COLOR_INPUT;

/// A diff stat with only its figures colored - additions green, deletions
/// red, the changed-file count blue - and the words and brackets muted, so
/// the numbers are what the eye lands on. Shared by the workspace header
/// and the dashboard cards so the two can't drift apart; the caller styles
/// the row's font and size.
///
/// The file noun comes from `l10n::plural_noun` rather than a local
/// `if count == 1`, and separately from the count so only the number takes
/// the accent color.
pub(crate) fn diff_stats_row(stats: &knot_git::DiffStats, muted: gpui_kit::Hsla) -> gpui_kit::Div {
    let files = knot_core::l10n::plural_noun(stats.files_changed, "count.file", "count.files");
    h_flex().flex_shrink_0()
            .whitespace_nowrap()
            .text_color(muted)
            .gap_1()
            .items_baseline()
            .child(div().text_color(rgb(DIFF_ADDED_COLOR))
                        .child(format!("+{}", stats.insertions)))
            .child(div().text_color(rgb(DIFF_REMOVED_COLOR))
                        .child(format!("-{}", stats.deletions)))
            .child(h_flex().items_baseline()
                           .child(div().child("("))
                           .child(div().text_color(rgb(DIFF_FILES_COLOR))
                                       .child(stats.files_changed.to_string()))
                           .child(div().ml_1().child(format!("{files})"))))
}

pub(crate) fn state_color(state: knot_agents::AgentState) -> gpui_kit::Hsla {
    match state {
        knot_agents::AgentState::Idle => rgb(consts::COLOR_IDLE).into(),
        knot_agents::AgentState::Running => rgb(consts::COLOR_RUNNING).into(),
        knot_agents::AgentState::Input => rgb(consts::COLOR_INPUT).into(),
        knot_agents::AgentState::Error => rgb(consts::COLOR_ERROR).into(),
    }
}

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
    OpenIn,
    MarkdownFiles,
    RegisterAgent,
    Deactivate,
    RestartAgent,
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
    pub(crate) const ALL: [Self; 14] = [Self::Separator,
                                        Self::NewCompanion,
                                        Self::NewShellCompanion,
                                        Self::EditAgent,
                                        Self::ForkAgent,
                                        Self::DuplicateAgent,
                                        Self::MoveToWorkspace,
                                        Self::SaveToBench,
                                        Self::OpenIn,
                                        Self::MarkdownFiles,
                                        Self::RegisterAgent,
                                        Self::Deactivate,
                                        Self::RestartAgent,
                                        Self::RemoveAgent];

    /// The user-visible label, or `None` for a separator. Matches the Swift
    /// reference's strings (`Skwad/Views/Components/AgentContextMenu.swift`),
    /// except that the port keeps "Remove Agent" where the reference says
    /// "Close Agent" - `agent-list-ui` already specifies the former.
    pub(crate) fn label(self) -> Option<&'static str> {
        match self {
            Self::Separator => None,
            Self::NewCompanion => Some("New Companion…"),
            Self::NewShellCompanion => Some("New Shell Companion"),
            Self::EditAgent => Some("Edit Agent…"),
            Self::ForkAgent => Some("Fork Agent"),
            Self::DuplicateAgent => Some("Duplicate Agent"),
            Self::MoveToWorkspace => Some("Move to Workspace"),
            Self::SaveToBench => Some("Save to Bench"),
            Self::OpenIn => Some("Open In…"),
            Self::MarkdownFiles => Some("Markdown Files"),
            Self::RegisterAgent => Some("Register Agent"),
            Self::Deactivate => Some("Deactivate"),
            Self::RestartAgent => Some("Restart Agent"),
            Self::RemoveAgent => Some("Remove Agent"),
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
                                               is_running:           true, };
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
    let groups = [vec![NewCompanion, NewShellCompanion].into_iter()
                                                       .filter(|_| owner_only)
                                                       .collect::<Vec<_>>(),
                  [EditAgent].into_iter()
                             .chain([ForkAgent, DuplicateAgent].into_iter()
                                                               .filter(|_| owner_only))
                             .collect(),
                  [MoveToWorkspace].into_iter()
                                   .filter(|_| owner_only && facts.has_move_targets)
                                   .chain([SaveToBench].into_iter().filter(|_| owner_only))
                                   .collect(),
                  [OpenIn].into_iter()
                          .chain([MarkdownFiles].into_iter()
                                                .filter(|_| facts.has_markdown_history))
                          .collect(),
                  // Deactivate sits with the other session actions, and
                  // immediately above Restart Agent: both act on the
                  // session rather than on the agent, and Deactivate is
                  // the reversible one of the pair.
                  [RegisterAgent].into_iter()
                                 .filter(|_| !facts.is_shell)
                                 .chain([Deactivate].into_iter().filter(|_| facts.is_running))
                                 .chain([RestartAgent].into_iter().filter(|_| owner_only))
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

/// One entry in the sidebar's background context menu - the menu that opens
/// on the agent list's empty space rather than on a row, scoped to the
/// workspace rather than to any one agent.
///
/// A separate enum from [`AgentMenuEntry`] rather than an extension of it:
/// the two menus share no item, and this one disables what does not apply
/// where the row's menu omits it, so nothing is gained by forcing both
/// through one type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentListBackgroundEntry {
    Separator,
    NewAgent,
    RestartAll,
    CloseAll,
    DeactivateAll,
    Broadcast,
}

impl AgentListBackgroundEntry {
    /// The user-visible label, or `None` for a separator. Matches the Swift
    /// reference's strings (`Skwad/Views/Sidebar/SidebarView.swift`), except
    /// Deactivate All, which the reference has no counterpart for.
    pub(crate) fn label(self) -> Option<&'static str> {
        match self {
            Self::Separator => None,
            Self::NewAgent => Some("New Agent"),
            Self::RestartAll => Some("Restart All"),
            Self::CloseAll => Some("Close All"),
            Self::DeactivateAll => Some("Deactivate All"),
            Self::Broadcast => Some("Broadcast to All Agents…"),
        }
    }
}

/// What the sidebar's background menu needs to know about the workspace it
/// opened over. Counts rather than the agents themselves: enablement is the
/// only decision this menu makes, and it turns on nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SidebarMenuFacts {
    /// Agents in the workspace the sidebar is showing.
    pub(crate) agent_count:   usize,
    /// How many of those are running, i.e. have a session to stop.
    pub(crate) running_count: usize,
}

/// One item of the sidebar's background menu: the entry and whether it
/// applies to this workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SidebarMenuItem {
    pub(crate) entry:   AgentListBackgroundEntry,
    /// Meaningless for [`AgentListBackgroundEntry::Separator`], which the
    /// renderer matches before it ever reads this.
    pub(crate) enabled: bool,
}

/// The sidebar background menu's entries, in order, with its one divider.
///
/// Unlike [`agent_context_menu_entries`], this returns every entry every
/// time and marks the ones that do not apply disabled. The row menu opens on
/// a different row each time and is read top to bottom; this one opens on the
/// same empty space every time and is learned by position, which an item set
/// that changes shape defeats (`agent-list-ui`).
pub(crate) fn sidebar_background_menu_entries(facts: SidebarMenuFacts) -> Vec<SidebarMenuItem> {
    use AgentListBackgroundEntry::*;

    let has_agents = facts.agent_count > 0;
    // Deactivate All needs a *running* agent, not merely an existing one:
    // Restart All and Close All apply to a stopped agent, stopping does not.
    let has_running = facts.running_count > 0;
    [(NewAgent, true),
     (RestartAll, has_agents),
     (CloseAll, has_agents),
     (DeactivateAll, has_running),
     (Separator, false),
     (Broadcast, has_agents)].into_iter()
                             .map(|(entry, enabled)| SidebarMenuItem { entry, enabled })
                             .collect()
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) struct LayoutModel {
    pub(crate) workspace_rows:      Vec<WorkspaceRow>,
    pub(crate) selected_agent_rows: Vec<AgentRow>,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn layout_model(store: &knot_agents::AgentStore, agent_selection: Option<Uuid>,
                           attached_ids: &[Uuid], unread_counts: &BTreeMap<Uuid, usize>)
                           -> LayoutModel {
    let current = store.current_workspace_id();
    let workspace_rows = store.workspaces()
                              .iter()
                              .map(|workspace| WorkspaceRow { id:       workspace.id,
                                                              name:     workspace.name.clone(),
                                                              selected: Some(workspace.id)
                                                                        == current, })
                              .collect::<Vec<_>>();

    let selected_agent_rows =
        store.workspaces()
             .iter()
             .find(|workspace| Some(workspace.id) == current)
             .map(|workspace| {
                 workspace.agent_ids
                          .iter()
                          .filter_map(|id| store.agent(*id))
                          .map(|agent| AgentRow { id:           agent.id,
                                                  avatar:       agent.avatar.clone(),
                                                  name:         agent.name.clone(),
                                                  agent_type:   agent.agent_type.clone(),
                                                  folder:       agent.folder.clone(),
                                                  selected:     Some(agent.id) == agent_selection,
                                                  attached:     attached_ids.contains(&agent.id),
                                                  state:        agent.state,
                                                  unread_count: unread_counts.get(&agent.id)
                                                                             .copied()
                                                                             .unwrap_or(0), })
                          .collect::<Vec<_>>()
             })
             .unwrap_or_default();

    LayoutModel { workspace_rows,
                  selected_agent_rows }
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn command_to_send(input: &str) -> Option<&str> {
    let command = input.trim();
    (!command.is_empty()).then_some(command)
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn stale_session_ids(session_ids: &[Uuid], live_ids: &BTreeSet<Uuid>) -> Vec<Uuid> {
    session_ids.iter()
               .copied()
               .filter(|id| !live_ids.contains(id))
               .collect()
}

/// Builds the agent store from persisted layout when
/// `restore_layout_on_launch` is set, otherwise starts empty. Shared between
/// the GPUI shell and the MCP catalog so both render the same data.
///
/// When `restore_conversation_on_launch` is also set, resolves each restored
/// agent's resume-session id: its own persisted session id when present (an
/// exact restore), otherwise the most recent session for its `(folder,
/// agent type)` via the `knot-history` provider registry.
pub(crate) fn build_agent_store(settings: &knot_core::Settings) -> knot_agents::AgentStore {
    if !settings.restore_layout_on_launch {
        return knot_agents::AgentStore::new();
    }

    let mut store = knot_agents::AgentStore::from_saved(&settings.saved_agents,
                                                        settings.saved_workspaces.clone());

    if settings.restore_conversation_on_launch {
        let persisted: BTreeMap<Uuid, String> =
            settings.saved_agents
                    .iter()
                    .filter_map(|agent| agent.session_id.clone().map(|sid| (agent.id, sid)))
                    .collect();
        store.resolve_resume_sessions(&persisted, |folder, agent_type| {
                 let provider = knot_history::provider(agent_type)?;
                 provider.load_sessions(folder)
                         .into_iter()
                         .next()
                         .map(|session| session.id)
             });
    }

    store
}

pub(crate) fn agent_selection_for_workspace(store: &knot_agents::AgentStore, workspace_id: Uuid)
                                            -> Option<Uuid> {
    let workspace = store.workspaces()
                         .iter()
                         .find(|workspace| workspace.id == workspace_id)?;
    workspace.active_agent_ids
             .iter()
             .chain(workspace.agent_ids.iter())
             .find(|id| store.agent(**id).is_some())
             .copied()
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn initial_agent_selection(store: &knot_agents::AgentStore) -> Option<Uuid> {
    store.current_workspace_id()
         .and_then(|id| agent_selection_for_workspace(store, id))
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
/// The slice of an agent the shell paints. [`agent_status_snapshot`] diffs
/// these so the poller only wakes the UI on visible changes, not on every
/// buffer append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentStatusKey {
    pub(crate) id:            Uuid,
    pub(crate) state:         knot_agents::AgentState,
    pub(crate) status_text:   String,
    pub(crate) is_registered: bool,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeliveryNotice {
    pub(crate) recipient_name: String,
    pub(crate) count:          usize,
}

// UNWIRED(#222): desktop-notifications' decision layer. Nothing calls
// `show_system_notification`, so this is reached only from tests.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AwaitingNotice {
    agent_name: String,
    message:    String,
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn delivery_notice(events: &[DeliveryEvent], agents: &[knot_agents::Agent])
                              -> Option<DeliveryNotice> {
    let event = events.last()?;
    let agent = agents.iter().find(|agent| agent.id == event.agent_id)?;
    let count = events.iter()
                      .filter(|event| event.agent_id == agent.id)
                      .count();
    Some(DeliveryNotice { recipient_name: agent.name.clone(),
                          count })
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn agent_status_snapshot(store: &knot_agents::AgentStore) -> Vec<AgentStatusKey> {
    store.agents()
         .iter()
         .map(|agent| AgentStatusKey { id:            agent.id,
                                       state:         agent.state,
                                       status_text:   agent.status_text.clone(),
                                       is_registered: agent.is_registered, })
         .collect()
}

// UNWIRED: ported from the Swift reference, no view reads it yet.
// Reached only from tests; kept as the port's staging area rather than
// deleted, so the behaviour it encodes is not lost.
#[allow(dead_code)]
pub(crate) fn unread_counts_snapshot(messages: &knot_messaging::MessageStore, agent_ids: &[Uuid])
                                     -> BTreeMap<Uuid, usize> {
    agent_ids.iter()
             .copied()
             .map(|id| (id, messages.unread_count(id)))
             .collect()
}

pub(crate) fn apply_terminal_status(store: &Arc<Mutex<knot_agents::AgentStore>>, agent_id: Uuid,
                                    state: knot_agents::AgentState) {
    {
        let mut store = store.lock();
        store.set_state(agent_id, state);
    }
}

/// What the idle-time delivery nudge needs to know about one agent, per
/// `mcp-messaging`. Grouped because the decision has six inputs and a
/// six-argument predicate invites callers to transpose two of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NudgeCheck<'a> {
    pub(crate) agent_type:     &'a str,
    pub(crate) mcp_enabled:    bool,
    /// The most recent unread message for this agent, if any.
    pub(crate) latest_message: Option<Uuid>,
    /// The last message this agent was already nudged about.
    pub(crate) last_nudged:    Option<Uuid>,
    /// The agent's state is Idle. A nudge must not land mid-work.
    pub(crate) idle:           bool,
    /// The agent has a live session able to take a prompt right now - for a
    /// panel agent, a ready slot with no turn in flight and no permission
    /// outstanding.
    pub(crate) can_receive:    bool,
}

// The bool form of `inbox_prompt_message_id`, kept because the spec's
// conditions read as a predicate and the tests assert them that way.
#[allow(dead_code)]
/// Whether to send `agent` the "check your inbox" prompt.
///
/// Every condition here is one the spec names: MCP off means no messaging at
/// all; a shell agent cannot receive messages; an unread message that has
/// already been nudged about must not nudge again on the next poll; and a
/// busy agent, or one with no live session, is nudged later instead.
pub(crate) fn should_inject_inbox_prompt(check: NudgeCheck<'_>) -> bool {
    inbox_prompt_message_id(check).is_some()
}

/// The message id an inbox nudge would be *about*, or `None` when no nudge is
/// due - the same decision [`should_inject_inbox_prompt`] reports as a bool,
/// carrying the value out with it.
///
/// Callers need both the verdict and the id they must record as nudged.
/// Returning it here rather than re-reading `latest_message` after a `true`
/// keeps the two from drifting: the caller used to `expect()` that the field
/// this predicate had checked was still `Some`, which is a panic in the
/// repaint poll the moment a condition above changes without the caller
/// changing with it.
pub(crate) fn inbox_prompt_message_id(check: NudgeCheck<'_>) -> Option<Uuid> {
    if !(check.mcp_enabled
         && check.agent_type != consts::SHELL_AGENT_TYPE
         && check.idle
         && check.can_receive)
    {
        return None;
    }
    check.latest_message
         .filter(|message_id| Some(*message_id) != check.last_nudged)
}

// UNWIRED(#222): desktop-notifications' decision layer. Nothing calls
// `show_system_notification`, so this is reached only from tests.
#[allow(dead_code)]
pub(crate) fn should_show_awaiting_notice(selected_agent: Option<Uuid>, agent_id: Uuid,
                                          message: &str, last_message: Option<&String>)
                                          -> bool {
    selected_agent != Some(agent_id)
    && !message.is_empty()
    && last_message.is_none_or(|last| last != message)
}

// UNWIRED(#222): desktop-notifications' decision layer. Nothing calls
// `show_system_notification`, so this is reached only from tests.
#[allow(dead_code)]
pub(crate) const AWAITING_INPUT_DEFAULT_BODY: &str = "Needs your attention";

// UNWIRED(#222): desktop-notifications' decision layer. Nothing calls
// `show_system_notification`, so this is reached only from tests.
#[allow(dead_code)]
/// Whether a desktop notification should be raised for an agent entering
/// Awaiting input, gating the same "is this a fresh prompt for an agent the
/// user isn't already looking at" signal `should_show_awaiting_notice`
/// computes for the in-window toast behind the
/// `desktop_notifications_enabled` setting.
pub(crate) fn should_notify(desktop_notifications_enabled: bool, show_awaiting_notice: bool)
                            -> bool {
    desktop_notifications_enabled && show_awaiting_notice
}

// UNWIRED(#222): desktop-notifications' decision layer. Nothing calls
// `show_system_notification`, so this is reached only from tests.
#[allow(dead_code)]
/// The notification body: the hook-supplied message when non-empty,
/// otherwise a default.
pub(crate) fn notification_body(message: &str) -> &str {
    if message.is_empty() {
        AWAITING_INPUT_DEFAULT_BODY
    }
    else {
        message
    }
}

/// Parses a [`SystemNotificationResponse`]'s tag back into the agent id it
/// was posted for, or `None` if the tag isn't a valid uuid.
///
/// The full "select this exact agent" click-to-navigate parity with the
/// Swift reference needs a registry mapping agent id -> owning workspace
/// window, which doesn't exist yet (each workspace is an independent
/// `Shell` window/entity with its own `agent_selection`, and nothing
/// currently tracks which window owns which agent across windows). Until
/// that exists, a click only raises the app to the front
/// (`App::activate(true)`, same as the existing `ShowAllWindows` action);
/// it doesn't switch the front window's selection to the clicked agent.
pub(crate) fn notification_response_agent_id(response: &SystemNotificationResponse)
                                             -> Option<Uuid> {
    Uuid::parse_str(&response.tag).ok()
}
