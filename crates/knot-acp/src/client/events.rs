use crate::error::SessionEndCause;
use crate::protocol::{ConfigOption, PermissionRequest, SessionUpdate};

#[derive(Debug)]
pub enum SessionEvent {
    Update(SessionUpdate),
    PermissionRequest(PermissionRequest),
    Ended(SessionEndCause),
}

#[derive(Debug, Clone, Default)]
pub struct NewSession {
    pub session_id:     String,
    pub config_options: Vec<ConfigOption>,
}
