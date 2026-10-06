//! Folds an ACP session's update stream into renderable panel state: the
//! message list with streaming text accumulation, tool-call cards, and
//! pending permission state. Sibling to `gpui_terminal::TerminalView` (which
//! renders a `Grid`) rather than a mode inside it - this folds a
//! `knot_acp::SessionEvent` stream into a different data model entirely.
//!
//! `message` is what the list is made of, `state` is what a session's panel
//! holds and what the UI asks of it, `fold` is how the event stream changes
//! it, and `summary` groups a turn's contiguous tool calls for compact mode.
//! `harness` tells a replayed user chunk the user typed from one the agent's
//! harness injected.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

mod fold;
mod harness;
mod message;
mod state;
mod summary;

pub use message::PanelMessage;
pub use message::ShellCard;
pub use message::ShellDelivery;
pub use message::ToolCallCard;
pub use state::PanelState;
pub use summary::ToolRunSummary;

#[cfg(test)]
mod tests;
