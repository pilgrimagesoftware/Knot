use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::Serialize;
use uuid::Uuid;

use crate::session::McpSessionManager;

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name:         String,
    pub description:  String,
    /// MCP names this field `inputSchema`; a client that validates
    /// `tools/list` against the schema drops every tool sent under the
    /// snake_case spelling, which is how Knot's whole tool set went
    /// missing from ACP-launched agents while a *differently named* MCP
    /// server in the user's own config still answered.
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: &'static str,
    pub properties:  BTreeMap<String, PropertySchema>,
    pub required:    Vec<String>,
}

impl Default for ToolInputSchema {
    fn default() -> Self {
        Self { schema_type: "object",
               properties:  BTreeMap::new(),
               required:    Vec::new(), }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PropertySchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub text:         String,
}

impl ToolContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self { content_type: "text",
               text:         text.into(), }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCallResult {
    pub content:  Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolCallResult {
    pub fn ok(text: impl Into<String>) -> Self {
        Self { content:  vec![ToolContent::text(text)],
               is_error: None, }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self { content:  vec![ToolContent::text(text)],
               is_error: Some(true), }
    }
}

/// Who a tool call comes from: the connection it arrived on, and the agent
/// that connection belongs to.
#[derive(Clone, Default)]
pub struct Caller {
    /// The agent the connection is bound to - named by its URL, or by the
    /// `register-agent` it made. `None` while it is bound to none.
    pub agent:      Option<Uuid>,
    /// The connection's MCP session, when the client keeps one; `None` for
    /// a client whose every request opens a fresh one.
    pub session_id: Option<String>,
    /// Every live connection, for asking which one holds an agent.
    pub sessions:   McpSessionManager,
}

impl Caller {
    /// Whether another live connection holds `agent_id`.
    pub fn held_elsewhere(&self, agent_id: Uuid) -> bool {
        self.sessions
            .held_elsewhere(agent_id, self.session_id.as_deref())
    }

    /// Binds this connection to `agent_id`, if the client keeps its session.
    pub fn bind(&self, agent_id: Uuid) {
        if let Some(id) = &self.session_id {
            self.sessions.bind(id, agent_id);
        }
    }
}

/// The set of MCP tools an `McpServer` dispatches `tools/list`/`tools/call`
/// through. The concrete catalog (`mcp-tools`) plugs in here without this
/// crate changing.
#[async_trait]
pub trait ToolCatalog: Send + Sync {
    fn list(&self) -> Vec<ToolDefinition>;
    async fn call(&self, name: &str, arguments: serde_json::Value) -> ToolCallResult;

    /// [`Self::call`], for `caller`. What the server dispatches through; a
    /// catalog that holds callers to who they are overrides it.
    async fn call_as(&self, name: &str, arguments: serde_json::Value, _caller: &Caller)
                     -> ToolCallResult {
        self.call(name, arguments).await
    }
}

/// A `ToolCatalog` with no tools; every call fails with "unknown tool".
pub struct EmptyCatalog;

#[async_trait]
impl ToolCatalog for EmptyCatalog {
    fn list(&self) -> Vec<ToolDefinition> {
        Vec::new()
    }

    async fn call(&self, name: &str, _arguments: serde_json::Value) -> ToolCallResult {
        ToolCallResult::error(format!("unknown tool: {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_catalog_reports_unknown_tool() {
        let result = EmptyCatalog.call("does-not-exist", serde_json::json!({}))
                                 .await;
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content.len(), 1);
        assert!(result.content[0].text.contains("does-not-exist"));
    }

    #[test]
    fn empty_catalog_lists_no_tools() {
        assert!(EmptyCatalog.list().is_empty());
    }
}
