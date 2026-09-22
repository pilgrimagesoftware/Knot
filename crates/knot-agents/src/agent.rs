use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use knot_core::{ActivationMode, ViewMode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Automatic state-machine state, driven by terminal activity and hooks.
/// Distinct from `Agent::status_text` (agent-set) and `Agent::terminal_title`
/// (terminal escape sequences).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    #[default]
    #[serde(rename = "Idle")]
    Idle,
    #[serde(rename = "Working")]
    Running,
    #[serde(rename = "Awaiting input")]
    Input,
    #[serde(rename = "Error")]
    Error,
}

/// The only view mode `agent_type` is allowed to hold: `Terminal` for a
/// shell agent (no ACP equivalent for a bare shell), `Panel` for every
/// other type (they launch exclusively through their ACP adapter - see
/// `agent-lifecycle`'s "View mode is fixed by agent type" requirement).
/// The single place that decides this, consulted by `create`, `edit`, and
/// `from_saved` so a non-shell agent can never end up in `Terminal`.
pub fn view_mode_for(agent_type: &str) -> ViewMode {
    if agent_type == "shell" {
        ViewMode::Terminal
    }
    else {
        ViewMode::Panel
    }
}

/// A running agent: the durable fields mirrored from `SavedAgent` plus
/// runtime-only state that resets to its default on every reload.
///
/// Build one with [`crate::convert::from_saved`], never by hand, so runtime
/// fields can't leak stale values from persistence.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    // Durable (mirrors `knot_core::SavedAgent`)
    pub id:              Uuid,
    pub name:            String,
    pub avatar:          String,
    pub folder:          String,
    pub agent_type:      String,
    pub created_by:      Option<Uuid>,
    pub is_companion:    bool,
    pub shell_command:   Option<String>,
    pub persona_id:      Option<Uuid>,
    pub view_mode:       ViewMode,
    /// When this agent's session starts on its own. Durable; distinct from
    /// [`Agent::activated`], which is runtime-only.
    pub activation_mode: ActivationMode,
    /// The Panel session setup last chosen for this agent - model,
    /// permission mode, reasoning effort - as adapter-declared ACP
    /// config-option id -> selected value. Durable, and deliberately not
    /// gated by `restore-conversation-on-launch`: it is a setup preference,
    /// not conversation content. See
    /// `openspec/specs/session-setup-persistence/spec.md`.
    pub session_config:  BTreeMap<String, String>,

    // Runtime-only
    /// Whether this agent has been activated in this run and so may start.
    /// Never persisted: a `Passive` agent that was running at quit comes
    /// back stopped, or "passive" would decay into "active after first use".
    pub activated:         bool,
    pub state:             AgentState,
    pub status_text:       String,
    pub is_registered:     bool,
    pub is_pending_start:  bool,
    pub terminal_title:    String,
    pub restart_token:     Uuid,
    pub session_id:        Option<String>,
    pub resume_session_id: Option<String>,
    pub fork_session:      bool,
    /// The ACP session id for a Panel-mode agent (`None` outside Panel
    /// mode, or before the first ACP session is created). Distinct from
    /// `session_id`, which is the terminal-resume identifier - ACP's
    /// `session/load` and a CLI's own `--resume <id>` are not always the
    /// same identifier space.
    pub acp_session_id:    Option<String>,
    pub metadata:          BTreeMap<String, String>,

    // Panel state, set by the `display-markdown` / `view-mermaid` MCP
    // tools. Not yet consumed by any UI.
    pub markdown_file:      Option<PathBuf>,
    pub markdown_maximized: bool,
    /// Most recent first.
    pub markdown_history:   Vec<PathBuf>,
    pub mermaid_source:     Option<String>,
    pub mermaid_title:      Option<String>,
}

impl Agent {
    /// Whether this is a plain shell agent (no AI).
    pub fn is_shell(&self) -> bool {
        self.agent_type == "shell"
    }

    /// The last component of the agent's folder, for the places that label
    /// an agent by where it works rather than by its full path - a sidebar
    /// row, a dashboard card. Falls back to the whole folder when there is
    /// no last component (a root path, or an empty string).
    pub fn folder_name(&self) -> String {
        Path::new(&self.folder).file_name()
                               .map(|name| name.to_string_lossy().into_owned())
                               .unwrap_or_else(|| self.folder.clone())
    }

    /// Title for the terminal header: prefers agent-set status text over the
    /// terminal-reported title.
    pub fn header_title(&self) -> &str {
        if self.status_text.is_empty() {
            &self.terminal_title
        }
        else {
            &self.status_text
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_state_serializes_to_swift_raw_strings() {
        assert_eq!(serde_json::to_string(&AgentState::Idle).unwrap(),
                   "\"Idle\"");
        assert_eq!(serde_json::to_string(&AgentState::Running).unwrap(),
                   "\"Working\"");
        assert_eq!(serde_json::to_string(&AgentState::Input).unwrap(),
                   "\"Awaiting input\"");
        assert_eq!(serde_json::to_string(&AgentState::Error).unwrap(),
                   "\"Error\"");
    }

    #[test]
    fn header_title_prefers_status_text() {
        let mut agent = test_agent();
        agent.status_text = "Refactoring auth".to_string();
        agent.terminal_title = "zsh".to_string();
        assert_eq!(agent.header_title(), "Refactoring auth");
    }

    #[test]
    fn header_title_falls_back_to_terminal_title() {
        let mut agent = test_agent();
        agent.status_text = String::new();
        agent.terminal_title = "zsh".to_string();
        assert_eq!(agent.header_title(), "zsh");
    }

    pub(crate) fn test_agent() -> Agent {
        Agent { id:                 Uuid::new_v4(),
                name:               "proj".to_string(),
                avatar:             "🤖".to_string(),
                folder:             "/tmp/proj".to_string(),
                agent_type:         "claude".to_string(),
                created_by:         None,
                is_companion:       false,
                shell_command:      None,
                persona_id:         None,
                view_mode:          ViewMode::Terminal,
                activation_mode:    ActivationMode::Passive,
                session_config:     BTreeMap::new(),
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
}
