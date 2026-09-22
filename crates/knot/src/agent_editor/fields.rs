//! The editor's form values: what a create derived from another agent starts
//! out as, what the pickers offer, and how raw field text becomes the agent's
//! own data.
//!
//! All of it is decided without the editor entity, so each piece is testable
//! on its own.

use uuid::Uuid;

/// What a new agent starts out as when the editor is opened to create one
/// *derived* from an existing agent - a fork, or a companion of it -
/// rather than from nothing. Mirrors the Swift reference's `AgentPrefill`
/// (`Skwad/Models/Agent.swift`). Every field is ignored when the editor
/// opens in edit mode.
#[derive(Debug, Clone, Default)]
pub(crate) struct AgentPrefill {
    pub(crate) name:         Option<String>,
    pub(crate) avatar:       Option<String>,
    /// Folder for the new agent, the way the Swift reference's
    /// `addAgent(to:)` does from a dashboard's "Add Agent" tile: copied
    /// from an existing agent in the workspace.
    pub(crate) folder:       Option<String>,
    pub(crate) agent_type:   Option<String>,
    pub(crate) persona_id:   Option<Uuid>,
    /// The owner, when creating a companion of an existing agent.
    pub(crate) created_by:   Option<Uuid>,
    pub(crate) is_companion: bool,
    /// The session the new agent continues, when forking. The created
    /// agent carries it as both its session id and its resume target, with
    /// the fork flag set, so the fork picks the conversation up rather than
    /// taking it over.
    pub(crate) session_id:   Option<String>,
}

/// The agent type a submitted create actually uses.
///
/// A companion is a shell agent by definition - `create_shell_companion`
/// hardcodes the type, and the MCP `create-agent` tool refuses `companion`
/// for anything else - so a companion-creating editor ignores whatever the
/// type field holds rather than creating a companion the rest of the stack
/// rejects.
pub(crate) fn created_agent_type(creating_a_companion: bool, chosen: &str) -> String {
    if creating_a_companion {
        knot_core::agent_type::SHELL.to_string()
    }
    else {
        chosen.to_string()
    }
}

/// The personas the editor's picker offers.
///
/// `Settings::personas` is the *stored* list, which keeps soft-deleted
/// system personas so persistence can tell "deleted" from "never
/// installed". Selection reads the active list instead, per
/// `openspec/specs/personas/spec.md` - "Active versus stored personas" -
/// which drops deleted entries and sorts by name.
pub(crate) fn persona_choices(settings: &knot_core::Settings) -> Vec<knot_core::Persona> {
    settings.active_personas().into_iter().cloned().collect()
}

/// Splits the capabilities field into tags. `Capabilities` trims,
/// lowercases and drops empties, so this only has to decide where one tag
/// ends and the next begins.
pub(crate) fn parse_capability_tags(raw: &str) -> knot_core::Capabilities {
    raw.split(',').collect()
}

/// What both submit paths take from the form once it is known to be
/// valid. Deliberately not the whole form: the fields each path reads
/// alone - a shell command when creating, a persona when editing - stay
/// where they are used.
pub(super) struct AgentFields {
    pub(super) folder: String,
    pub(super) name:   String,
    pub(super) avatar: String,
}
