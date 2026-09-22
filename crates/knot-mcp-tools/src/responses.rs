//! Success-payload structs, one per tool, matching the Swift reference's
//! `MCPTypes.swift` JSON shape (`#[serde(rename_all = "camelCase")]` so
//! Rust's `snake_case` fields serialize to the same wire keys).

use serde::Serialize;

/// One agent (or deployable template) as a caller sees it.
///
/// Carries the registry fields alongside the identity ones so a listing is
/// self-sufficient: everything needed to choose a candidate, without a
/// second call to interpret it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id:            String,
    pub name:          String,
    pub folder:        String,
    /// The automatic state for a live agent; `template` for a bench entry,
    /// which has no session and so no state to report.
    pub status:        String,
    pub is_registered: bool,
    pub description:   String,
    pub capabilities:  Vec<String>,
    pub tools:         Vec<String>,
    pub cost_tier:     String,
}

#[derive(Debug, Serialize)]
pub struct DescribeAgentsResponse {
    pub candidates: Vec<AgentInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub id:            String,
    pub goal:          String,
    pub state:         String,
    /// The agent this task names, or the capability tags standing in for
    /// one until dispatch resolves them. Absent when the plan recorded the
    /// work without deciding who does it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee:      Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub capabilities:  Vec<String>,
    pub depends_on:    Vec<String>,
    /// The agent a dispatch actually went to, once one has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispatched_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlanTasksResponse {
    pub tasks: Vec<TaskInfo>,
}

#[derive(Debug, Serialize)]
pub struct TaskStatusResponse {
    pub tasks: Vec<TaskInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchTaskResponse {
    pub success:      bool,
    pub task_id:      String,
    pub recipient_id: String,
    pub message:      String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteTaskResponse {
    pub success:     bool,
    pub task_id:     String,
    pub state:       String,
    /// Tasks this outcome made dispatchable, and tasks it stopped.
    pub now_ready:   Vec<String>,
    pub now_blocked: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentInfo>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CheckMessagesResponse {
    pub messages: Vec<MessageInfo>,
}

#[derive(Debug, Serialize)]
pub struct MessageInfo {
    pub id:        String,
    pub from:      String,
    pub content:   String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastResponse {
    pub success:         bool,
    pub recipient_count: usize,
}

#[derive(Debug, Serialize)]
pub struct RepoInfoResponse {
    pub name:      String,
    pub worktrees: Vec<WorktreeInfoResponse>,
}

#[derive(Debug, Serialize)]
pub struct ListReposResponse {
    pub repos: Vec<RepoInfoResponse>,
}

#[derive(Debug, Serialize)]
pub struct WorktreeInfoResponse {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorktreesResponse {
    pub repo_path: String,
    pub worktrees: Vec<WorktreeInfoResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentResponse {
    pub success:  bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub message:  String,
}

#[derive(Debug, Serialize)]
pub struct CloseAgentResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterAgentResponse {
    pub success:              bool,
    pub message:              String,
    pub unread_message_count: usize,
    pub knot_members:         Vec<AgentInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorktreeResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path:    Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ShowMarkdownResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ShowMermaidResponse {
    pub success: bool,
    pub message: String,
}

/// Pretty-prints `value` into a [`knot_mcp::ToolCallResult`], matching the
/// Swift reference's `successResult` (pretty-printed JSON as the tool's text
/// content).
pub fn success<T: Serialize>(value: &T) -> knot_mcp::ToolCallResult {
    match serde_json::to_string_pretty(value) {
        Ok(text) => knot_mcp::ToolCallResult::ok(text),
        Err(err) => knot_mcp::ToolCallResult::error(format!("Failed to encode result: {err}")),
    }
}
