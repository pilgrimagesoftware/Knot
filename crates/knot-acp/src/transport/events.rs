use serde_json::Value;

use crate::error::SessionEndCause;

#[derive(Debug, Clone)]
pub enum TransportEvent {
    Request {
        id: Value,
        method: String,
        params: Option<Value>,
    },
    Notification {
        method: String,
        params: Option<Value>,
    },
    Ended(SessionEndCause),
}
