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
        kind:         String,
    },
    ToolCallUpdate {
        tool_call_id: String,
        status:       String,
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
            Some("agent_message_chunk") | Some("text_delta") => {
                SessionUpdate::TextDelta { text: text_content(&update), }
            }
            Some("tool_call") => {
                SessionUpdate::ToolCallStart { tool_call_id: field_str(&update, "toolCallId"),
                                               kind:         field_str(&update, "kind"), }
            }
            Some("tool_call_update") => {
                SessionUpdate::ToolCallUpdate { tool_call_id: field_str(&update, "toolCallId"),
                                                status:       field_str(&update, "status"), }
            }
            Some("tool_call_result") => {
                SessionUpdate::ToolCallResult { tool_call_id: field_str(&update, "toolCallId"),
                                                output:       update.get("output")
                                                                    .cloned()
                                                                    .unwrap_or(Value::Null), }
            }
            Some("diff") => SessionUpdate::Diff { path: field_str(&update, "path"),
                                                  diff: field_str(&update, "diff"), },
            Some("turn_end") => {
                SessionUpdate::TurnEnd { stop_reason: field_str(&update, "stopReason"), }
            }
            Some("config_option_update") => {
                let config_options = update.get("configOptions")
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

fn field_str(value: &Value, key: &str) -> String {
    value.get(key)
         .and_then(Value::as_str)
         .unwrap_or_default()
         .to_owned()
}

/// A `session/request_permission` request from the agent, awaiting an
/// allow/deny decision from the caller.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRequest {
    pub rpc_id:       Value,
    pub tool_call_id: String,
    pub options:      Vec<PermissionOption>,
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
mod tests {
    use super::*;

    #[test]
    fn text_delta_parses_from_the_real_nested_update_envelope() {
        // Captured against a live `gemini --acp` handshake (task 1.2):
        // the notification's params nest the typed update, and the text
        // itself is a content block, not a flat field.
        let params = serde_json::json!({
            "sessionId": "s1",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "pong" }
            }
        });

        let update = SessionUpdate::from_params(params);

        assert!(matches!(update, SessionUpdate::TextDelta { text } if text == "pong"));
    }

    #[test]
    fn text_delta_still_parses_the_flat_legacy_shape() {
        let params = serde_json::json!({ "sessionUpdate": "text_delta", "text": "hi" });

        let update = SessionUpdate::from_params(params);

        assert!(matches!(update, SessionUpdate::TextDelta { text } if text == "hi"));
    }

    #[test]
    fn unrecognized_nested_update_kind_is_unknown() {
        let params = serde_json::json!({
            "sessionId": "s1",
            "update": { "sessionUpdate": "available_commands_update", "availableCommands": [] }
        });

        let update = SessionUpdate::from_params(params.clone());

        assert!(matches!(update, SessionUpdate::Unknown { raw } if raw == params));
    }
}
