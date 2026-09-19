//! The only place durable/runtime `Agent` fields cross: `from_saved` builds
//! a fresh `Agent` with runtime fields at their documented defaults,
//! `to_saved` reads only the eight durable fields back out.

use std::collections::BTreeMap;

use knot_core::SavedAgent;
use uuid::Uuid;

use crate::agent::{Agent, AgentState, view_mode_for};

/// Build a runtime agent from its persisted record. Runtime fields always
/// start at their documented defaults, never copied from anywhere. The
/// persisted `view_mode` is coerced by `agent_type` rather than trusted
/// verbatim, since a legacy/edited record could carry a mismatched value
/// (see `view_mode_for`).
pub fn from_saved(saved: &SavedAgent) -> Agent {
    Agent { id:              saved.id,
            name:            saved.name.clone(),
            avatar:          saved.avatar.clone(),
            folder:          saved.folder.clone(),
            agent_type:      saved.agent_type.clone(),
            created_by:      saved.created_by,
            is_companion:    saved.is_companion,
            shell_command:   saved.shell_command.clone(),
            persona_id:      saved.persona_id,
            view_mode:       view_mode_for(&saved.agent_type),
            activation_mode: saved.activation_mode,

            activated:          false,
            state:              AgentState::Idle,
            status_text:        String::new(),
            is_registered:      false,
            is_pending_start:   false,
            terminal_title:     String::new(),
            restart_token:      Uuid::new_v4(),
            session_id:         None,
            resume_session_id:  None,
            fork_session:       false,
            acp_session_id:     None,
            metadata:           BTreeMap::new(),
            markdown_file:      None,
            markdown_maximized: false,
            markdown_history:   Vec::new(),
            mermaid_source:     None,
            mermaid_title:      None, }
}

/// Extract the durable subset of a runtime agent for persistence.
/// `remember_conversation` gates session id and ACP session id: only
/// carried into the saved record when true (`restore-conversation-on-launch`
/// enabled), otherwise always persisted as `None` regardless of the agent's
/// runtime values.
pub fn to_saved(agent: &Agent, remember_conversation: bool) -> SavedAgent {
    SavedAgent { id:              agent.id,
                 name:            agent.name.clone(),
                 avatar:          agent.avatar.clone(),
                 folder:          agent.folder.clone(),
                 agent_type:      agent.agent_type.clone(),
                 created_by:      agent.created_by,
                 is_companion:    agent.is_companion,
                 shell_command:   agent.shell_command.clone(),
                 persona_id:      agent.persona_id,
                 view_mode:       agent.view_mode,
                 activation_mode: agent.activation_mode,
                 session_id:      remember_conversation.then(|| agent.session_id.clone())
                                                       .flatten(),
                 acp_session_id:  remember_conversation.then(|| agent.acp_session_id.clone())
                                                       .flatten(), }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `agent_type` defaults to `claude` (non-shell), so `view_mode` is set
    /// to what `view_mode_for` coerces it to on load - keeps the round-trip
    /// test below meaningful without re-deriving the coercion itself.
    fn saved_agent() -> SavedAgent {
        let mut saved = SavedAgent::new(Uuid::new_v4(), "proj", None, "/tmp/proj");
        saved.view_mode = crate::agent::view_mode_for(&saved.agent_type);
        saved
    }

    #[test]
    fn from_saved_drops_runtime_state() {
        // A record that, in the Swift reference, had accumulated runtime state
        // before being persisted (runtime fields are never serialized, so this
        // is really pinning that `from_saved` never invents non-default state).
        let saved = saved_agent();

        let agent = from_saved(&saved);

        assert_eq!(agent.state, AgentState::Idle);
        assert!(!agent.is_registered);
        assert_eq!(agent.session_id, None);
        assert_eq!(agent.terminal_title, "");
    }

    #[test]
    fn to_saved_from_saved_round_trips_durable_fields() {
        let saved = saved_agent();

        let agent = from_saved(&saved);
        let back = to_saved(&agent, false);

        assert_eq!(saved, back);
    }

    #[test]
    fn to_saved_carries_session_id_only_when_remembering() {
        let saved = saved_agent();
        let mut agent = from_saved(&saved);
        agent.session_id = Some("s7".to_string());

        assert_eq!(to_saved(&agent, true).session_id, Some("s7".to_string()));
        assert_eq!(to_saved(&agent, false).session_id, None);
    }

    #[test]
    fn view_mode_round_trips_through_save_and_load() {
        let saved = saved_agent();

        let agent = from_saved(&saved);

        assert_eq!(agent.view_mode, knot_core::ViewMode::Panel);
        assert_eq!(to_saved(&agent, false).view_mode,
                   knot_core::ViewMode::Panel);
    }

    #[test]
    fn non_shell_agent_always_loads_as_panel_even_if_persisted_as_terminal() {
        let mut saved = saved_agent();
        saved.agent_type = "claude".to_string();
        saved.view_mode = knot_core::ViewMode::Terminal;

        let agent = from_saved(&saved);

        assert_eq!(agent.view_mode, knot_core::ViewMode::Panel);
    }

    #[test]
    fn shell_agent_always_loads_as_terminal_even_if_persisted_as_panel() {
        let mut saved = saved_agent();
        saved.agent_type = "shell".to_string();
        saved.view_mode = knot_core::ViewMode::Panel;

        let agent = from_saved(&saved);

        assert_eq!(agent.view_mode, knot_core::ViewMode::Terminal);
    }

    #[test]
    fn acp_session_id_resets_on_load_and_carries_only_when_remembering() {
        let mut saved = saved_agent();
        saved.acp_session_id = Some("acp-1".to_string());

        // Per the agent-lifecycle spec: acp_session_id is conditionally
        // durable, like session_id - the runtime value always starts at
        // None after a load, regardless of what was persisted.
        let mut agent = from_saved(&saved);
        assert_eq!(agent.acp_session_id, None);

        agent.acp_session_id = Some("acp-2".to_string());
        assert_eq!(to_saved(&agent, true).acp_session_id,
                   Some("acp-2".to_string()));
        assert_eq!(to_saved(&agent, false).acp_session_id, None);
    }

    #[test]
    fn a_running_passive_agent_reloads_passive_and_unactivated() {
        let mut saved = saved_agent();
        saved.activation_mode = knot_core::ActivationMode::Passive;

        let mut agent = from_saved(&saved);
        agent.activated = true;

        let reloaded = from_saved(&to_saved(&agent, false));

        assert_eq!(reloaded.activation_mode, knot_core::ActivationMode::Passive);
        assert!(!reloaded.activated);
    }

    #[test]
    fn legacy_record_without_companion_fields_loads_with_defaults() {
        let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                           Uuid::new_v4());
        let saved: SavedAgent = serde_json::from_str(&json).unwrap();

        let agent = from_saved(&saved);

        assert_eq!(agent.created_by, None);
        assert!(!agent.is_companion);
        assert_eq!(agent.agent_type, "claude");
    }
}
