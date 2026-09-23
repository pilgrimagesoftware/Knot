//! The serialized record types stored inside [`super::Settings`]: personas,
//! saved agents, bench templates, and workspaces. Field names use the Swift
//! `CodingKeys` (camelCase) so a document written by either implementation
//! round-trips. `#[serde(default)]` on the later-added fields is the
//! decode-tolerant migration path.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::consts::{DEFAULT_AGENT_TYPE, DEFAULT_AVATAR};
use crate::settings::capabilities::Capabilities;
use crate::settings::vocabulary::CostTier;

// ---------------------------------------------------------------------------
// Persona
// ---------------------------------------------------------------------------

/// Origin of a persona: shipped with the app or created by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersonaType {
    System,
    User,
}

/// Lifecycle state. `Deleted` is a soft delete so a removed system persona is
/// not reinstalled on the next launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PersonaState {
    Enabled,
    Disabled,
    Deleted,
}

/// A named instruction block an agent can adopt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Persona {
    pub id:           Uuid,
    pub name:         String,
    pub instructions: String,
    #[serde(rename = "type", default = "default_persona_type")]
    pub persona_type: PersonaType,
    #[serde(default = "default_persona_state")]
    pub state:        PersonaState,
}

/// Which surface an agent is driven through: the terminal grid (default,
/// every agent type), or a native ACP panel for agent types with a
/// registered adapter (`knot-agent-launch::acp_adapter`). The terminal is
/// never removed - Panel mode only changes which view/launch path is
/// active, per `openspec/specs/acp-panel-ui/spec.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    #[default]
    Terminal,
    Panel,
}

/// When an agent's session starts on its own: an `Active` agent starts with
/// its workspace, a `Passive` one waits to be selected. See
/// `openspec/specs/agent-lifecycle/spec.md` - "Activation mode".
///
/// `Passive` is the enum's own default because that is what the new-agent
/// dialog offers; the *load* default is deliberately different, see
/// [`default_activation_mode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    Active,
    #[default]
    Passive,
}

// ---------------------------------------------------------------------------
// SavedAgent
// ---------------------------------------------------------------------------

/// A persisted agent. Runtime-only fields are not stored; loading reconstructs
/// them at their defaults.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAgent {
    pub id:              Uuid,
    pub name:            String,
    #[serde(default = "default_avatar")]
    pub avatar:          String,
    pub folder:          String,
    #[serde(default = "default_agent_type")]
    pub agent_type:      String,
    #[serde(default)]
    pub created_by:      Option<Uuid>,
    #[serde(default)]
    pub is_companion:    bool,
    #[serde(default)]
    pub shell_command:   Option<String>,
    #[serde(default)]
    pub persona_id:      Option<Uuid>,
    #[serde(default)]
    pub session_id:      Option<String>,
    #[serde(default)]
    pub view_mode:       ViewMode,
    #[serde(default)]
    pub acp_session_id:  Option<String>,
    /// Loads as `Active`, not the enum's own `Passive` default: a record
    /// written before this field existed describes an agent that started
    /// with its workspace, and it must go on doing so. New agents get
    /// `Passive` from `CreateOptions` instead.
    #[serde(default = "default_activation_mode")]
    pub activation_mode: ActivationMode,
    /// Registry metadata: what this agent is for, what it can be asked to
    /// do, and how expensive it is to ask. Absent from records written
    /// before the registry existed, which load undescribed, untagged and
    /// mid-priced rather than being hidden. See
    /// `openspec/specs/agent-registry/spec.md`.
    #[serde(default)]
    pub description:     String,
    #[serde(default)]
    pub capabilities:    Capabilities,
    #[serde(default)]
    pub cost_tier:       CostTier,
    /// The session setup last chosen in the Panel - model, permission mode
    /// and reasoning effort - keyed by the ACP config-option id the adapter
    /// declared, holding that option's selected value.
    ///
    /// Not an enum per closed-vocabulary convention, and not three named
    /// fields: the ids and their legal values are declared by the adapter at
    /// runtime (`ConfigOption`), not fixed by Knot, and they differ between
    /// agent types. Storing the adapter's own id/value pairs is what lets the
    /// selection be replayed verbatim on reopen. Absent for records written
    /// before this field existed, which restore the adapter's own defaults.
    /// See `openspec/specs/session-setup-persistence/spec.md`.
    #[serde(default)]
    pub session_config:  BTreeMap<String, String>,
}

impl SavedAgent {
    /// Build a saved agent, substituting the default robot avatar when `avatar`
    /// is `None` or empty. Remaining fields start at their defaults; set them
    /// on the returned value as needed.
    pub fn new(id: Uuid, name: impl Into<String>, avatar: Option<String>,
               folder: impl Into<String>)
               -> Self {
        let avatar = avatar.filter(|a| !a.is_empty())
                           .unwrap_or_else(default_avatar);
        Self { id,
               name: name.into(),
               avatar,
               folder: folder.into(),
               agent_type: default_agent_type(),
               created_by: None,
               is_companion: false,
               shell_command: None,
               persona_id: None,
               session_id: None,
               view_mode: ViewMode::default(),
               acp_session_id: None,
               activation_mode: default_activation_mode(),
               description: String::new(),
               capabilities: Capabilities::new(),
               cost_tier: CostTier::default(),
               session_config: BTreeMap::new() }
    }
}

// ---------------------------------------------------------------------------
// BenchAgent
// ---------------------------------------------------------------------------

/// A reusable agent template on the bench.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchAgent {
    pub id:            Uuid,
    pub name:          String,
    #[serde(default = "default_avatar")]
    pub avatar:        String,
    pub folder:        String,
    #[serde(default = "default_agent_type")]
    pub agent_type:    String,
    #[serde(default)]
    pub shell_command: Option<String>,
    #[serde(default)]
    pub persona_id:    Option<Uuid>,
    /// Registry metadata carried onto the agent this template deploys, so a
    /// saved template records a role and not just a folder.
    #[serde(default)]
    pub description:   String,
    #[serde(default)]
    pub capabilities:  Capabilities,
    #[serde(default)]
    pub cost_tier:     CostTier,
}

impl BenchAgent {
    /// Build a bench entry with default avatar/agent-type fallback.
    pub fn new(id: Uuid, name: impl Into<String>, avatar: Option<String>,
               folder: impl Into<String>)
               -> Self {
        let avatar = avatar.filter(|a| !a.is_empty())
                           .unwrap_or_else(default_avatar);
        Self { id,
               name: name.into(),
               avatar,
               folder: folder.into(),
               agent_type: default_agent_type(),
               shell_command: None,
               persona_id: None,
               description: String::new(),
               capabilities: Capabilities::new(),
               cost_tier: CostTier::default() }
    }
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

/// A saved group of agents: what the user configured about it, and nothing
/// more. How its window was last arranged is [`WorkspaceUiState`], in its own
/// document - see `openspec/specs/settings-persistence/spec.md`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id:        Uuid,
    pub name:      String,
    pub color_hex: String,
    #[serde(default)]
    pub agent_ids: Vec<Uuid>,
}

/// What the application recorded about how one workspace's window was last
/// arranged. The user never entered any of it and would not miss it if it
/// were discarded, which is why it is not part of [`Workspace`]: where a
/// window sits is the most frequently written value in the store and the
/// least valuable, and it must not be a reason to rewrite the roster.
///
/// The port stores the layout fields as opaque data; it does not interpret
/// them here.
///
/// Every field carries its own serde default, so an entry written before a
/// field existed loads with that field defaulted rather than failing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceUiState {
    pub layout_mode:           String,
    pub active_agent_ids:      Vec<Uuid>,
    pub focused_pane_index:    i32,
    pub split_ratio:           f64,
    pub split_ratio_secondary: Option<f64>,
    pub show_dashboard:        Option<bool>,
    pub is_detached:           Option<bool>,
    /// Where this workspace's window was last seen, so reopening it puts
    /// it back rather than re-centring. `None` until the window has been
    /// opened once.
    pub window_bounds:         Option<SavedWindowBounds>,
}

/// A window's position and size in logical pixels, as plain numbers -
/// `knot-core` has no GPUI dependency, so the UI converts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SavedWindowBounds {
    pub x:      f32,
    pub y:      f32,
    pub width:  f32,
    pub height: f32,
}

// ---------------------------------------------------------------------------
// serde defaults
// ---------------------------------------------------------------------------

pub(super) fn default_avatar() -> String {
    DEFAULT_AVATAR.to_string()
}

pub(super) fn default_agent_type() -> String {
    DEFAULT_AGENT_TYPE.to_string()
}

/// The load default for [`SavedAgent::activation_mode`]. Deliberately not
/// `ActivationMode::default()`: see the field's comment.
fn default_activation_mode() -> ActivationMode {
    ActivationMode::Active
}

fn default_persona_type() -> PersonaType {
    PersonaType::User
}

fn default_persona_state() -> PersonaState {
    PersonaState::Enabled
}

// ---------------------------------------------------------------------------
// SavedPullRequest
// ---------------------------------------------------------------------------

/// A pull request Knot saw in an agent's output.
///
/// The record is evidence of what was seen, not a copy of the pull request:
/// no title, no number, no status. Those are fetched and refreshed, because a
/// merged pull request shown as open after a restart is worse than a blank,
/// and there is no way to know a remembered status is still true.
///
/// Identity is the agent plus the canonical URL: the same pull request seen by
/// two agents is two records, because who opened it is part of what is
/// recorded.
///
/// The Swift reference has no counterpart, so the field names here are chosen
/// rather than inherited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPullRequest {
    /// The canonical URL, as `knot_core::pull_request_url::PullRequestUrl`
    /// renders it - never the decorated form the output carried, so two
    /// spellings of one pull request are one record.
    pub url:          String,
    /// The agent whose output carried the URL.
    pub agent_id:     Uuid,
    /// The workspace that agent belonged to when the URL was seen. Held on
    /// the record rather than looked up through the agent, so a list can be
    /// built for a workspace without walking every agent.
    pub workspace_id: Uuid,
    /// When the URL was first seen, in seconds since the Unix epoch.
    ///
    /// Seconds rather than a formatted timestamp because the only thing the
    /// store does with it is order rows newest-first, and an integer cannot
    /// sort wrong across time zones the way a string can. Formatting it for
    /// a person is the view's business.
    pub first_seen:   i64,
}

impl SavedPullRequest {
    /// A record of `url`, seen now, against `agent_id` in `workspace_id`.
    #[must_use]
    pub fn new(url: impl Into<String>, agent_id: Uuid, workspace_id: Uuid) -> Self {
        Self { url: url.into(),
               agent_id,
               workspace_id,
               first_seen: now_unix() }
    }

    /// Whether this record is the same pull request seen by the same agent -
    /// which is what makes recording idempotent.
    #[must_use]
    pub fn is_same_sighting(&self, url: &str, agent_id: Uuid) -> bool {
        self.agent_id == agent_id && self.url == url
    }
}

/// Seconds since the Unix epoch, or `0` if the clock is set before it.
fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                .map_or(0, |since| {
                                    i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
                                })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn persona_enum_round_trips_lowercase() {
        assert_eq!(serde_json::to_string(&PersonaType::System).unwrap(),
                   "\"system\"");
        assert_eq!(serde_json::to_string(&PersonaState::Deleted).unwrap(),
                   "\"deleted\"");
        let parsed: PersonaState = serde_json::from_str("\"enabled\"").unwrap();
        assert_eq!(parsed, PersonaState::Enabled);
    }

    #[test]
    fn legacy_persona_defaults_type_and_state() {
        let json =
            r#"{"id":"a1000001-0000-0000-0000-000000000001","name":"X","instructions":"do X"}"#;
        let persona: Persona = serde_json::from_str(json).unwrap();
        assert_eq!(persona.persona_type, PersonaType::User);
        assert_eq!(persona.state, PersonaState::Enabled);
    }

    #[test]
    fn legacy_saved_agent_defaults_added_fields() {
        let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                           id());
        let agent: SavedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(agent.agent_type, "claude");
        assert_eq!(agent.created_by, None);
        assert!(!agent.is_companion);
        assert_eq!(agent.persona_id, None);
        assert_eq!(agent.session_id, None);
        assert!(agent.session_config.is_empty());
    }

    #[test]
    fn saved_agent_session_config_round_trips() {
        let mut agent = SavedAgent::new(id(), "A", None, "/tmp");
        agent.session_config
             .insert("model".to_string(), "sonnet".to_string());
        agent.session_config
             .insert("permission_mode".to_string(), "plan".to_string());
        agent.session_config
             .insert("reasoning-effort".to_string(), "high".to_string());

        let json = serde_json::to_string(&agent).unwrap();
        assert!(json.contains("sessionConfig"), "camelCase key: {json}");

        let decoded: SavedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.session_config, agent.session_config);
    }

    #[test]
    fn legacy_saved_agent_without_activation_mode_loads_active() {
        let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                           id());
        let agent: SavedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(agent.activation_mode, ActivationMode::Active);
    }

    #[test]
    fn activation_mode_round_trips_in_lowercase() {
        assert_eq!(serde_json::to_string(&ActivationMode::Passive).unwrap(),
                   "\"passive\"");
        let mut a = SavedAgent::new(id(), "A", None, "/tmp");
        a.activation_mode = ActivationMode::Passive;
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"activationMode\":\"passive\""), "{json}");
        let back: SavedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn saved_agent_session_id_round_trips() {
        let mut a = SavedAgent::new(id(), "A", None, "/tmp");
        a.session_id = Some("s7".to_string());
        let json = serde_json::to_string(&a).unwrap();
        let back: SavedAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.session_id, Some("s7".to_string()));
    }

    #[test]
    fn saved_agent_new_defaults_avatar() {
        let a = SavedAgent::new(id(), "A", None, "/tmp");
        assert_eq!(a.avatar, DEFAULT_AVATAR);
        let b = SavedAgent::new(id(), "B", Some(String::new()), "/tmp");
        assert_eq!(b.avatar, DEFAULT_AVATAR);
        let c = SavedAgent::new(id(), "C", Some("🦀".to_string()), "/tmp");
        assert_eq!(c.avatar, "🦀");
    }

    #[test]
    fn legacy_bench_agent_defaults() {
        let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                           id());
        let bench: BenchAgent = serde_json::from_str(&json).unwrap();
        assert_eq!(bench.agent_type, "claude");
        assert_eq!(bench.persona_id, None);
    }

    #[test]
    fn workspace_round_trips() {
        let ws = Workspace { id:        id(),
                             name:      "Main".to_string(),
                             color_hex: "#1B4FB2".to_string(),
                             agent_ids: vec![id()], };
        let json = serde_json::to_string(&ws).unwrap();
        let back: Workspace = serde_json::from_str(&json).unwrap();
        assert_eq!(ws, back);
    }

    #[test]
    fn workspace_ui_state_round_trips() {
        let ui = WorkspaceUiState { layout_mode:           "grid".to_string(),
                                    active_agent_ids:      vec![id()],
                                    focused_pane_index:    1,
                                    split_ratio:           0.5,
                                    split_ratio_secondary: Some(0.3),
                                    show_dashboard:        Some(false),
                                    is_detached:           Some(true),
                                    window_bounds:         Some(SavedWindowBounds { x:      12.,
                                                                                    y:      34.,
                                                                                    width:  960.,
                                                                                    height: 640., }), };
        let json = serde_json::to_string(&ui).unwrap();
        let back: WorkspaceUiState = serde_json::from_str(&json).unwrap();
        assert_eq!(ui, back);
    }

    /// The wire keys are what an existing `workspaces.json` was written with,
    /// so the split can only find them under these exact names.
    #[test]
    fn workspace_ui_state_keeps_the_camel_case_keys_it_was_stored_under() {
        let json = serde_json::to_value(WorkspaceUiState::default()).unwrap();
        let object = json.as_object().unwrap();
        for key in ["layoutMode",
                    "activeAgentIds",
                    "focusedPaneIndex",
                    "splitRatio",
                    "splitRatioSecondary",
                    "showDashboard",
                    "isDetached",
                    "windowBounds"]
        {
            assert!(object.contains_key(key), "UI state missing key {key}");
        }
    }

    // --- SavedPullRequest --------------------------------------------------

    #[test]
    fn saved_pull_request_round_trips_in_camel_case() {
        let record = SavedPullRequest { url:
                                            "https://github.com/acme/widget/pull/42".to_string(),
                                        agent_id:     id(),
                                        workspace_id: id(),
                                        first_seen:   1_758_566_400, };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"agentId\""), "camelCase key: {json}");
        assert!(json.contains("\"workspaceId\""), "camelCase key: {json}");
        assert!(json.contains("\"firstSeen\""), "camelCase key: {json}");

        let back: SavedPullRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    /// The record is evidence of a sighting, not a copy of the pull request.
    /// A status persisted here would be shown as current after a restart
    /// when it is not.
    #[test]
    fn saved_pull_request_carries_no_fetched_state() {
        let json = serde_json::to_string(&SavedPullRequest::new("https://github.com/a/b/pull/1",
                                                                id(),
                                                                id())).unwrap();

        for absent in ["title", "number", "state", "status", "isDraft"] {
            assert!(!json.contains(absent),
                    "{absent} must not be persisted: {json}");
        }
    }

    #[test]
    fn a_new_saved_pull_request_is_stamped_with_the_current_time() {
        let record = SavedPullRequest::new("https://github.com/a/b/pull/1", id(), id());

        // 2026-01-01, comfortably in the past whenever this runs.
        assert!(record.first_seen > 1_767_225_600, "{}", record.first_seen);
    }

    /// Idempotence is per agent: the same URL from a second agent is a second
    /// record, because who opened it is part of what is recorded.
    #[test]
    fn a_sighting_matches_only_the_same_url_from_the_same_agent() {
        let agent = id();
        let other_agent = id();
        let record = SavedPullRequest::new("https://github.com/a/b/pull/1", agent, id());

        assert!(record.is_same_sighting("https://github.com/a/b/pull/1", agent));
        assert!(!record.is_same_sighting("https://github.com/a/b/pull/2", agent));
        assert!(!record.is_same_sighting("https://github.com/a/b/pull/1", other_agent));
    }

    /// UI state saved before window frames were remembered must still load -
    /// the field is absent from every entry written before it existed.
    #[test]
    fn ui_state_without_saved_window_bounds_still_loads() {
        let ui: WorkspaceUiState = serde_json::from_str(r#"{"layoutMode":"single"}"#).unwrap();

        assert_eq!(ui.layout_mode, "single");
        assert_eq!(ui.window_bounds, None);
    }

    /// A workspace record still carrying the UI keys - every existing
    /// `workspaces.json` - must load as a workspace, because the split reads
    /// the document as JSON and decodes each record after lifting them out.
    #[test]
    fn a_combined_workspace_record_still_decodes_as_a_workspace() {
        let json = r##"{"id":"00000000-0000-0000-0000-000000000001","name":"Main",
                        "colorHex":"#1B4FB2","layoutMode":"grid","splitRatio":0.25}"##;

        let ws: Workspace = serde_json::from_str(json).unwrap();

        assert_eq!(ws.name, "Main");
        assert!(ws.agent_ids.is_empty());
    }
}
