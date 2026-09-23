//! Local MCP HTTP server.
//!
//! Implements `openspec/specs/mcp-server/spec.md`.

mod consts;
mod error;
mod hooks;
mod log;
mod rpc;
mod server;
mod session;
mod status;
mod tools;

pub use consts::{DEFAULT_PORT, LOG_FILE_NAME};
pub use error::{McpError, Result};
pub use hooks::{
    AgentHookHandler, HookError, HookRequest, HookStatus, claude_status, codex_turn_complete,
    extract_metadata, last_assistant_message_from_transcript,
};
pub use log::{Entry, Level, Logger, Subject};
pub use rpc::{JsonRpcError, JsonRpcId, JsonRpcRequest, JsonRpcResponse, dispatch};
pub use server::{AgentsSnapshotFn, McpServer};
pub use session::{McpSession, McpSessionManager};
pub use status::{AgentStatusEntry, agent_status};
pub use tools::{
    EmptyCatalog, PropertySchema, ToolCallResult, ToolCatalog, ToolContent, ToolDefinition,
    ToolInputSchema,
};
