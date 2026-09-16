//! JSON-RPC 2.0 envelope types and ACP-specific payload shapes.
//!
//! Only the fields `knot-acp` actually reads/writes are modeled; unknown
//! fields on incoming messages are ignored (`serde(default)` /
//! passthrough), since the agent's response may carry ACP fields this
//! client doesn't yet use.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The ACP protocol major version this client speaks. Per `acp-client`'s
/// capability-negotiation requirement, a mismatched agent version fails the
/// connection rather than guessing at compatibility.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: &'static str,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcErrorPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorPayload {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// A message read off the subprocess's stdout: an id-bearing response to a
/// request we sent, an id-bearing request the agent is sending us
/// (e.g. `session/request_permission`), or an id-less notification
/// (e.g. `session/update`).
#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage {
    #[serde(default)]
    pub id: Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcErrorPayload>,
}

impl IncomingMessage {
    /// A server-to-client request (has both an id and a method).
    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    /// A notification (no id).
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    /// A response to a request we sent (has an id, no method).
    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    #[serde(default)]
    pub capabilities: AgentCapabilities,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentCapabilities {
    #[serde(default, rename = "loadSession")]
    pub supports_resume: bool,
    #[serde(default, rename = "permissionModes")]
    pub permission_modes: Vec<String>,
}

/// A decoded `session/update` notification, per `acp-client`'s streaming
/// requirement: text deltas, tool-call lifecycle, diffs, and turn-end are
/// distinguished, not flattened into one opaque payload.
#[derive(Debug, Clone)]
pub enum SessionUpdate {
    TextDelta {
        text: String,
    },
    ToolCallStart {
        tool_call_id: String,
        kind: String,
    },
    ToolCallUpdate {
        tool_call_id: String,
        status: String,
    },
    ToolCallResult {
        tool_call_id: String,
        output: Value,
    },
    Diff {
        path: String,
        diff: String,
    },
    TurnEnd {
        stop_reason: String,
    },
    Unknown {
        raw: Value,
    },
}

impl SessionUpdate {
    pub fn from_params(params: Value) -> Self {
        let kind = params.get("sessionUpdate").and_then(Value::as_str);
        match kind {
            Some("agent_message_chunk") | Some("text_delta") => SessionUpdate::TextDelta {
                text: params
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            },
            Some("tool_call") => SessionUpdate::ToolCallStart {
                tool_call_id: field_str(&params, "toolCallId"),
                kind: field_str(&params, "kind"),
            },
            Some("tool_call_update") => SessionUpdate::ToolCallUpdate {
                tool_call_id: field_str(&params, "toolCallId"),
                status: field_str(&params, "status"),
            },
            Some("tool_call_result") => SessionUpdate::ToolCallResult {
                tool_call_id: field_str(&params, "toolCallId"),
                output: params.get("output").cloned().unwrap_or(Value::Null),
            },
            Some("diff") => SessionUpdate::Diff {
                path: field_str(&params, "path"),
                diff: field_str(&params, "diff"),
            },
            Some("turn_end") => SessionUpdate::TurnEnd {
                stop_reason: field_str(&params, "stopReason"),
            },
            _ => SessionUpdate::Unknown { raw: params },
        }
    }
}

fn field_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A `session/request_permission` request from the agent, awaiting an
/// allow/deny decision from the caller.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub rpc_id: Value,
    pub tool_call_id: String,
    pub options: Vec<PermissionOption>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionOption {
    #[serde(rename = "optionId")]
    pub option_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    Allow,
    Deny,
}
