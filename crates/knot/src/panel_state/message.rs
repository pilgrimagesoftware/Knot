//! What the panel's message list is made of: one entry per prompt, streamed
//! response, tool-call card or reported failure.

use knot_acp::ToolCallContent;
use serde_json::Value;

/// Pretty-prints a legacy `tool_call_result` payload for display, falling
/// back to its compact form if it somehow can't be re-serialized.
pub(super) fn render_json(output: &Value) -> String {
    serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
}

/// One entry in the panel's message list: a user-sent prompt, streamed
/// assistant text, a tool call's card, or a failure the session reported
/// out of band - kept as distinct variants per `acp-panel-ui`'s "visually
/// distinguish user messages, assistant messages, and system/tool
/// content" requirement.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelMessage {
    User(String),
    Assistant(String),
    ToolCall(ToolCallCard),
    /// Something the session could not do: a refused prompt, a send that
    /// never reached the agent. These arrive as a JSON-RPC error response
    /// rather than a session update, so nothing in the event stream
    /// records them - without this entry the only trace of, say, an
    /// exhausted model quota was a line on stderr the user never sees.
    Error(String),
}

/// A tool call's rendered state: `kind` (execute, read, edit, ...), the
/// agent's human-readable `title`, its lifecycle `status`, and whatever
/// content has arrived so far - per the "Turn ends mid tool call"
/// scenario, the last known state is kept, never dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallCard {
    pub id:      String,
    pub kind:    String,
    pub title:   String,
    /// `pending`, `in_progress`, `completed` or `failed` - defaulted to
    /// `pending` at start rather than left unknown, per the ACP spec.
    pub status:  String,
    /// Output blocks as they arrive. A later update's `content` replaces
    /// this wholesale (the spec's updates carry the full current content,
    /// not a delta); an update with no `content` at all leaves it alone.
    pub content: Vec<ToolCallContent>,
}

impl ToolCallCard {
    /// Whether the call has finished, either way - the renderer shows an
    /// in-progress placeholder only while this is false.
    pub fn is_finished(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "failed")
    }

    // Completes the status trio with `is_finished`/`succeeded`; the renderer
    // happens to need only the other two.
    #[allow(dead_code)]
    pub fn failed(&self) -> bool {
        self.status == "failed"
    }

    /// Whether the call finished without failing - the one status that
    /// collapses by default, per `acp-panel-ui`'s "A finished tool call
    /// collapses to its header" requirement.
    pub fn succeeded(&self) -> bool {
        self.status == "completed"
    }
}
