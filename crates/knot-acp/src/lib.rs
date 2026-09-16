//! ACP (Agent Client Protocol) client: JSON-RPC 2.0 over a subprocess's
//! stdio, capability negotiation, session lifecycle, streaming updates, and
//! permission-request handling for driving an ACP-speaking agent as a
//! structured session instead of a character stream.
//!
//! Contract: `openspec/specs/acp-client/spec.md`. Agent-agnostic - no
//! knowledge of `Agent`, GPUI, or per-agent-type launch rules lives here
//! (see `knot-agent-launch`'s adapter registry for that).

mod client;
mod error;
mod protocol;
mod transport;

pub use client::{AcpClient, SessionEvent};
pub use error::{AcpError, Result, SessionEndCause};
pub use protocol::{AgentCapabilities, PermissionDecision, PermissionOption, PermissionRequest, SessionUpdate};
