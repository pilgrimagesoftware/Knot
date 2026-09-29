use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id:      i64,
    pub method:  String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params:  Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: &'static str,
    pub method:  String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params:  Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id:      Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error:   Option<JsonRpcErrorPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorPayload {
    pub code:    i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:    Option<Value>,
}

impl JsonRpcErrorPayload {
    /// `message`, followed by `data.details` when the agent sent one.
    /// `claude-agent-acp` answers a failed `session/new` with a bare
    /// "Internal error" and puts the reason - the CLI's own stderr, such as
    /// an option it did not know - in `details`.
    pub fn described(&self) -> String {
        match self.data
                  .as_ref()
                  .and_then(|data| data.get("details"))
                  .and_then(Value::as_str)
        {
            Some(details) if !details.is_empty() => format!("{}: {details}", self.message),
            _ => self.message.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncomingMessage {
    #[serde(default)]
    pub id:     Option<Value>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error:  Option<JsonRpcErrorPayload>,
}

impl IncomingMessage {
    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }

    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn payload(data: Option<Value>) -> JsonRpcErrorPayload {
        JsonRpcErrorPayload { code: -32603,
                              message: "Internal error".to_owned(),
                              data }
    }

    #[test]
    fn details_follow_the_message() {
        let error = payload(Some(json!({ "details": "unknown option '--typo'" })));
        assert_eq!(error.described(), "Internal error: unknown option '--typo'");
    }

    #[test]
    fn without_details_the_message_stands_alone() {
        assert_eq!(payload(None).described(), "Internal error");
        assert_eq!(payload(Some(json!({ "other": 1 }))).described(),
                   "Internal error");
    }
}
