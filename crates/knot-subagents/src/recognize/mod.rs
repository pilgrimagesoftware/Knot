//! Turning one agent's reporting into a [`SubagentEvent`].
//!
//! Contract: `openspec/specs/agent-subagents/spec.md` - "Subagents reach Knot
//! by the agent's own reporting".
//!
//! One recognizer per agent type, because the wire shape is the adapter's,
//! not the protocol's: ACP says a tool call has a `kind` and a `title` and
//! stops there, so every adapter identifies its own tools differently. A
//! recognizer is a pure function from a report to an optional event, which is
//! what lets the whole surface be tested against committed fixtures.
//!
//! **Recognition never errors.** A report this crate does not understand
//! answers `None`. An agent issues far more ordinary tool calls than
//! delegations, so "not a delegation" is the common case rather than a
//! failure, and a `Result` here would make every caller unwrap a non-event.
//!
//! [`SubagentEvent`]: crate::event::SubagentEvent

pub mod claude;
pub mod hooks;
pub mod recognizer;
pub mod report;

pub use claude::ClaudeRecognizer;
pub use hooks::recognize_hook;
pub use recognizer::{Recognizer, for_agent_type};
pub use report::ToolCallReport;
