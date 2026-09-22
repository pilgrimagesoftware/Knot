//! Folds an ACP session's update stream into renderable panel state: the
//! message list with streaming text accumulation, tool-call cards, and
//! pending permission state. Sibling to `terminal_view.rs` (which renders
//! a `Grid`) rather than a mode inside it - this folds a
//! `knot_acp::SessionEvent` stream into a different data model entirely.
//!
//! `message` is what the list is made of, `state` is what a session's panel
//! holds and what the UI asks of it, `fold` is how the event stream changes
//! it, and `summary` groups a turn's contiguous tool calls for compact mode.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

mod fold;
mod message;
mod state;
mod summary;

pub use message::PanelMessage;
pub use message::ToolCallCard;
pub use state::PanelState;
pub use summary::ToolRunSummary;

#[cfg(test)]
mod tests;
