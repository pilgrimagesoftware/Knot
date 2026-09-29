//! JSON-RPC 2.0 envelope types and ACP-specific payload shapes.
//!
//! Only the fields `knot-acp` actually reads/writes are modeled; unknown
//! fields on incoming messages are ignored (`serde(default)` /
//! passthrough), since the agent's response may carry ACP fields this
//! client doesn't yet use.

mod json_rpc;

pub use json_rpc::{
    IncomingMessage, JsonRpcErrorPayload, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The ACP protocol major version this client speaks. Per `acp-client`'s
/// capability-negotiation requirement, a mismatched agent version fails the
/// connection rather than guessing at compatibility.
pub const PROTOCOL_VERSION: u32 = 1;

/// The name Knot's own MCP server is declared under in `session/new`, and
/// therefore the name an agent sees it by.
///
/// Public because the registration prompt has to name it: an agent loads
/// its own user-level MCP configuration on top of this one, and a server
/// there can expose identically named tools. The prompt telling the agent
/// which server is its knot is only correct if it says the same name this
/// handshake sends, so both read this constant.
pub const MCP_SERVER_NAME: &str = "knot";

#[derive(Debug, Clone, Serialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    // The real field is "agentCapabilities", not "capabilities" -
    // confirmed against a live `gemini --acp` handshake (task 1.2); the
    // previous key silently defaulted to Default::default() (false/empty)
    // on every real agent instead of erroring, since #[serde(default)]
    // masks a rename mistake the same way it masks a genuinely absent
    // field.
    #[serde(default, rename = "agentCapabilities")]
    pub capabilities:     AgentCapabilities,
    /// Session Config Options some agents declare on `initialize` rather
    /// than (or in addition to) `session/new` - both locations are parsed
    /// the same way, since the stabilized spec allows either.
    #[serde(default, rename = "configOptions")]
    pub config_options:   Vec<ConfigOption>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentCapabilities {
    #[serde(default, rename = "loadSession")]
    pub supports_resume:  bool,
    #[serde(default, rename = "permissionModes")]
    pub permission_modes: Vec<String>,
    #[serde(default, rename = "mcpCapabilities")]
    pub mcp_capabilities: McpCapabilities,
}

/// Which MCP server transports the agent accepts in `session/new`'s
/// `mcpServers` list - per the spec, every agent MUST support `stdio`
/// (not modeled here, since Knot's own MCP server is HTTP-only); `http`
/// and `sse` are opt-in and default to unsupported.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct McpCapabilities {
    #[serde(default)]
    pub http: bool,
    #[serde(default)]
    pub sse:  bool,
}

/// One agent-declared session setting (mode, model, reasoning effort, ...)
/// per ACP's stabilized Session Config Options mechanism - a `select`-type
/// option renders as a picker; other declared `type`s (e.g. `boolean`) are
/// parsed but left unrendered until a control needs them.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct ConfigOption {
    pub id:            String,
    pub name:          String,
    #[serde(default)]
    pub category:      Option<String>,
    #[serde(default, rename = "type")]
    pub kind:          String,
    #[serde(default, rename = "currentValue")]
    pub current_value: Value,
    #[serde(default)]
    pub options:       Vec<ConfigOptionValue>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConfigOptionValue {
    pub value:       String,
    pub name:        String,
    #[serde(default)]
    pub description: Option<String>,
}

/// One entry of a tool call's `content` array, per the "Content" section
/// of ACP's tool-call docs
/// (<https://agentclientprotocol.com/protocol/tool-calls>). Results and
/// diffs ride on `tool_call`/`tool_call_update` rather than arriving as
/// `sessionUpdate` kinds of their own, so this is the only path by which
/// a tool call's output reaches the caller.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolCallContent {
    /// A `{"type": "content", "content": {...}}` block, flattened to its
    /// text (non-text blocks - images, resources - carry no text and are
    /// surfaced as an empty string rather than dropped silently).
    Text(String),
    /// A `{"type": "diff", ...}` block: a file modification. `old_text` is
    /// absent for a newly created file.
    Diff {
        path:     String,
        old_text: Option<String>,
        new_text: String,
    },
    /// A `{"type": "terminal", "terminalId": ...}` block - a live terminal
    /// embedded in the tool call. Knot has no terminal-sharing client
    /// capability yet, so only the id is carried.
    Terminal { terminal_id: String },
}

impl ToolCallContent {
    /// Parses a tool call's `content` array, skipping entries whose `type`
    /// this client doesn't model. Absent or non-array `content` (the
    /// common case for an update that only changes `status`) yields an
    /// empty vector, which callers MUST treat as "no change" rather than
    /// "cleared".
    fn parse_list(value: Option<&Value>) -> Vec<Self> {
        value.and_then(Value::as_array)
             .map(|entries| entries.iter().filter_map(Self::parse_one).collect())
             .unwrap_or_default()
    }

    fn parse_one(entry: &Value) -> Option<Self> {
        match entry.get("type").and_then(Value::as_str) {
            Some("content") => Some(Self::Text(entry.get("content")
                                                    .map(text_content_block)
                                                    .unwrap_or_default())),
            Some("diff") => Some(Self::Diff { path:     field_str(entry, "path"),
                                              old_text: entry.get("oldText")
                                                             .and_then(Value::as_str)
                                                             .map(str::to_owned),
                                              new_text: field_str(entry, "newText"), }),
            Some("terminal") => {
                Some(Self::Terminal { terminal_id: field_str(entry, "terminalId"), })
            }
            _ => None,
        }
    }
}

/// A decoded `session/update` notification, per `acp-client`'s streaming
/// requirement: text deltas, tool-call lifecycle, diffs, and turn-end are
/// distinguished, not flattened into one opaque payload.
#[derive(Debug, Clone)]
pub enum SessionUpdate {
    Usage {
        used: u64,
        size: u64,
    },
    TextDelta {
        text: String,
    },
    /// A piece of one of the user's own messages. Agents send these when
    /// `session/load` replays a conversation, so the client can show the
    /// prompts between the replies it replays as text deltas.
    ///
    /// Not every one was typed by the user: Claude Code stores what its
    /// harness injects - a `<task-notification>` when a background task
    /// ends - as a user message, and `claude-agent-acp` replays it the same
    /// way. `meta` is the chunk's `_meta` envelope, verbatim, so a client can
    /// read an origin tag (`_claude/origin`) where an adapter sends one.
    UserMessageChunk {
        text: String,
        meta: Option<Value>,
    },
    ToolCallStart {
        tool_call_id: String,
        kind:         String,
        /// The agent's human-readable label for the call ("Reading
        /// configuration file"), which is what the spec intends a client
        /// to show - `kind` is only an icon/category hint.
        title:        String,
        /// Defaults to `pending` per the spec when the agent omits it.
        status:       String,
        content:      Vec<ToolCallContent>,
        /// The call's `rawInput`, verbatim, or `None` when the agent sent
        /// none - distinguishable from an empty object.
        ///
        /// `kind` is an icon hint and `title` is prose written for a human,
        /// so neither says what tool ran. Adapters answer that out of band,
        /// here and in `meta`, and a client that cannot see the two fields
        /// cannot recognize a particular call at all.
        raw_input:    Option<Value>,
        /// The call's `_meta` envelope, verbatim, vendor keys included.
        meta:         Option<Value>,
    },
    /// Every field but `tool_call_id` is optional in an update, per the
    /// spec's "only the fields being changed need to be included" - hence
    /// `Option`, so an update that carries only `content` doesn't blank
    /// out a status the client already knows.
    ToolCallUpdate {
        tool_call_id: String,
        status:       Option<String>,
        title:        Option<String>,
        content:      Vec<ToolCallContent>,
        /// As on [`SessionUpdate::ToolCallStart`]. `None` means this update
        /// did not carry one, which a consumer MUST treat as "no change"
        /// rather than "cleared" - the spec's "only the fields being changed
        /// need to be included" applies to these as much as to `status`.
        raw_input:    Option<Value>,
        meta:         Option<Value>,
    },
    ToolCallResult {
        tool_call_id: String,
        output:       Value,
    },
    Diff {
        path: String,
        diff: String,
    },
    TurnEnd {
        stop_reason: String,
    },
    /// A `config_option_update` push - the full current set of Session
    /// Config Options, e.g. after `session/set_config_option` changes one
    /// option's value or an agent-initiated change elsewhere.
    ConfigOptionUpdate {
        config_options: Vec<ConfigOption>,
    },
    Unknown {
        raw: Value,
    },
}

impl SessionUpdate {
    pub fn from_params(params: Value) -> Self {
        // The real wire format nests the typed update under an "update"
        // envelope (`{"sessionId": ..., "update": {"sessionUpdate": ...,
        // ...}}`), confirmed against a live `gemini --acp` handshake -
        // `params` itself is accepted too, for callers that already
        // unwrapped it (and for existing tests).
        let update = params.get("update")
                           .cloned()
                           .unwrap_or_else(|| params.clone());
        let kind = update.get("sessionUpdate").and_then(Value::as_str);
        match kind {
            Some("usage_update") => SessionUpdate::Usage {
                used: update.get("used").and_then(Value::as_u64).unwrap_or_default(),
                size: update.get("size").and_then(Value::as_u64).unwrap_or_default(),
            },
            Some("agent_message_chunk") | Some("text_delta") => SessionUpdate::TextDelta {
                text: text_content(&update),
            },
            Some("user_message_chunk") => SessionUpdate::UserMessageChunk {
                text: text_content(&update),
                meta: update.get("_meta").cloned(),
            },
            Some("tool_call") => SessionUpdate::ToolCallStart {
                tool_call_id: field_str(&update, "toolCallId"),
                kind: optional_str(&update, "kind").unwrap_or_else(|| "other".to_string()),
                title: field_str(&update, "title"),
                status: optional_str(&update, "status").unwrap_or_else(|| "pending".to_string()),
                content: ToolCallContent::parse_list(update.get("content")),
                raw_input: update.get("rawInput").cloned(),
                meta: update.get("_meta").cloned(),
            },
            Some("tool_call_update") => SessionUpdate::ToolCallUpdate {
                tool_call_id: field_str(&update, "toolCallId"),
                status: optional_str(&update, "status"),
                title: optional_str(&update, "title"),
                content: ToolCallContent::parse_list(update.get("content")),
                raw_input: update.get("rawInput").cloned(),
                meta: update.get("_meta").cloned(),
            },
            Some("tool_call_result") => SessionUpdate::ToolCallResult {
                tool_call_id: field_str(&update, "toolCallId"),
                output: update.get("output").cloned().unwrap_or(Value::Null),
            },
            Some("diff") => SessionUpdate::Diff {
                path: field_str(&update, "path"),
                diff: field_str(&update, "diff"),
            },
            Some("turn_end") => SessionUpdate::TurnEnd {
                stop_reason: field_str(&update, "stopReason"),
            },
            Some("config_option_update") => {
                let config_options = update
                    .get("configOptions")
                    .cloned()
                    .map(serde_json::from_value)
                    .and_then(std::result::Result::ok)
                    .unwrap_or_default();
                SessionUpdate::ConfigOptionUpdate { config_options }
            }
            _ => SessionUpdate::Unknown { raw: params },
        }
    }
}

/// A text update's content, per ACP's content-block shape
/// (`{"content": {"type": "text", "text": "..."}}`), falling back to a
/// flat `text` field for other/older producers.
fn text_content(update: &Value) -> String {
    update.get("content")
          .and_then(|content| content.get("text"))
          .or_else(|| update.get("text"))
          .and_then(Value::as_str)
          .unwrap_or_default()
          .to_owned()
}

/// A bare content block's text (`{"type": "text", "text": "..."}`), as
/// opposed to [`text_content`], which digs the block out of an update
/// envelope first.
fn text_content_block(block: &Value) -> String {
    block.get("text")
         .and_then(Value::as_str)
         .unwrap_or_default()
         .to_owned()
}

fn field_str(value: &Value, key: &str) -> String {
    optional_str(value, key).unwrap_or_default()
}

/// `field_str` for a field whose absence is meaningful - an update that
/// omits `status` is leaving it unchanged, not clearing it.
fn optional_str(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// A `session/request_permission` request from the agent, awaiting an
/// allow/deny decision from the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRequest {
    pub rpc_id:          Value,
    pub tool_call_id:    String,
    pub tool_call_title: Option<String>,
    pub options:         Vec<PermissionOption>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PermissionOption {
    #[serde(rename = "optionId")]
    pub option_id: String,
    pub name:      String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}

#[cfg(test)]
mod tests;
