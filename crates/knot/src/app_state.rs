use super::*;
#[derive(Debug, PartialEq)]
pub(crate) struct WorkspaceRow {
    pub(crate) id:       Uuid,
    pub(crate) name:     String,
    pub(crate) selected: bool,
}

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
pub(crate) fn state_color(state: knot_agents::AgentState) -> gpui_kit::Hsla {
    match state {
        knot_agents::AgentState::Idle => rgb(0x22C55E).into(),
        knot_agents::AgentState::Running => rgb(0xF97316).into(),
        knot_agents::AgentState::Input => rgb(0x3B82F6).into(),
        knot_agents::AgentState::Error => rgb(0xEF4444).into(),
    }
}

/// The agent-row context menu's item labels, gated on whether the row is a
/// shell companion - a companion can't own companions or restart
/// independently of its owner (`agent-lifecycle`), so its menu omits those
/// two items. Pure so the item set is unit-testable independent of GPUI.
pub(crate) fn agent_context_menu_items(is_companion: bool) -> Vec<&'static str> {
    let mut items = vec!["Edit Agent…"];
    if !is_companion {
        items.push("New Shell Companion");
        items.push("Restart Agent");
    }
    items.push("Remove Agent");
    items
}

#[derive(Debug, PartialEq)]
pub(crate) struct LayoutModel {
    pub(crate) workspace_rows:      Vec<WorkspaceRow>,
    pub(crate) selected_agent_rows: Vec<AgentRow>,
}

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

pub(crate) fn command_to_send(input: &str) -> Option<&str> {
    let command = input.trim();
    (!command.is_empty()).then_some(command)
}

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

pub(crate) fn initial_agent_selection(store: &knot_agents::AgentStore) -> Option<Uuid> {
    store.current_workspace_id()
         .and_then(|id| agent_selection_for_workspace(store, id))
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeliveryNotice {
    pub(crate) recipient_name: String,
    pub(crate) count:          usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AwaitingNotice {
    agent_name: String,
    message:    String,
}

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

pub(crate) fn agent_status_snapshot(store: &knot_agents::AgentStore) -> Vec<AgentStatusKey> {
    store.agents()
         .iter()
         .map(|agent| AgentStatusKey { id:            agent.id,
                                       state:         agent.state,
                                       status_text:   agent.status_text.clone(),
                                       is_registered: agent.is_registered, })
         .collect()
}

pub(crate) fn unread_counts_snapshot(messages: &knot_messaging::MessageStore, agent_ids: &[Uuid])
                                     -> BTreeMap<Uuid, usize> {
    agent_ids.iter()
             .copied()
             .map(|id| (id, messages.unread_count(id)))
             .collect()
}

pub(crate) fn apply_terminal_status(store: &Arc<Mutex<knot_agents::AgentStore>>, agent_id: Uuid,
                                    state: knot_agents::AgentState) {
    if let Ok(mut store) = store.lock() {
        store.set_state(agent_id, state);
    }
}

pub(crate) fn should_inject_inbox_prompt(agent_type: &str, mcp_enabled: bool,
                                         latest_message: Option<Uuid>,
                                         last_injected: Option<Uuid>)
                                         -> bool {
    mcp_enabled
    && agent_type != "shell"
    && latest_message.is_some_and(|message_id| Some(message_id) != last_injected)
}

pub(crate) fn should_show_awaiting_notice(selected_agent: Option<Uuid>, agent_id: Uuid,
                                          message: &str, last_message: Option<&String>)
                                          -> bool {
    selected_agent != Some(agent_id)
    && !message.is_empty()
    && last_message.is_none_or(|last| last != message)
}

pub(crate) const AWAITING_INPUT_DEFAULT_BODY: &str = "Needs your attention";

/// Whether a desktop notification should be raised for an agent entering
/// Awaiting input, gating the same "is this a fresh prompt for an agent the
/// user isn't already looking at" signal `should_show_awaiting_notice`
/// computes for the in-window toast behind the
/// `desktop_notifications_enabled` setting.
pub(crate) fn should_notify(desktop_notifications_enabled: bool, show_awaiting_notice: bool)
                            -> bool {
    desktop_notifications_enabled && show_awaiting_notice
}

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
