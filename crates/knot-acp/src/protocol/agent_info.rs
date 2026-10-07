//! The agent's `agentInfo` from the `initialize` response.
//!
//! ACP's "Implementation Information": the agent SHOULD send a `name`, a
//! `version` and optionally a human-readable `title`
//! (<https://agentclientprotocol.com/protocol/initialization#implementation-information>).
//! "SHOULD", not "MUST" - the spec notes it becomes required in a future
//! version - so an agent may omit it, and the connection must not fail when
//! it does.

use serde::Deserialize;
use serde::Deserializer;
use serde_json::Value;

/// Who answered `initialize`: the adapter or agent implementation, and its
/// version.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentInfo {
    /// Intended for programmatic use. The display fallback when `title` is
    /// absent.
    pub name:    String,
    /// Intended for the UI. Preferred over `name` when present.
    #[serde(default)]
    pub title:   Option<String>,
    /// The implementation's version. Modeled as optional even though the
    /// spec's `Implementation` always carries it: an agent that sends a
    /// name without one still says which agent it is.
    #[serde(default)]
    pub version: Option<String>,
}

impl AgentInfo {
    /// The name to show a user: `title` if the agent gave a non-blank one,
    /// otherwise `name`, as the spec directs.
    pub fn display_name(&self) -> &str {
        self.title
            .as_deref()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or(&self.name)
    }

    /// The version, if the agent gave a non-blank one.
    pub fn version(&self) -> Option<&str> {
        self.version
            .as_deref()
            .filter(|version| !version.trim().is_empty())
    }
}

/// Deserializes an optional `agentInfo`, dropping one that does not parse.
///
/// A plain `Option<AgentInfo>` would fail the whole `initialize` response,
/// and with it the connection, over a malformed field the client only
/// displays. Absent, `null` and malformed all read as `None`.
pub(super) fn lenient<'de, D>(deserializer: D) -> Result<Option<AgentInfo>, D::Error>
    where D: Deserializer<'de> {
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}
