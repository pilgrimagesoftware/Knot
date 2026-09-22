//! Renders `panel_state::PanelState` as a chat-like panel: streaming
//! messages, tool-call cards (icon by ACP `kind`, body from the call's
//! reported content, with a diff view for diff blocks), and an inline
//! permission prompt. Sibling to `terminal_view.rs` (which renders a
//! `Grid`) per design decision 5 - this renders a completely different
//! data model.
//!
//! `style` and `callbacks` are what a render is handed, `render` is the tree
//! itself, and `message`, `rows`, `summary_row` and `tool_call` are the pieces
//! it puts together.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

mod callbacks;
mod message;
mod render;
mod rows;
mod style;
mod summary_row;
mod tool_call;

// What `workspace_window` reaches for: it drives the virtualized list through
// `rows`, builds a `PanelStyle` and `PanelCallbacks` per frame, and calls
// `render_panel` with them. Everything else here is internal to the module.
pub(crate) use callbacks::PanelCallbacks;
pub(crate) use render::LIST_OVERDRAW;
pub(crate) use render::render_panel;
pub(crate) use rows::*;
pub(crate) use style::PanelStyle;
pub(crate) use style::RiskLevel;
pub(crate) use style::permission_risk_level;
pub(crate) use style::risk_color;

#[cfg(test)]
mod tests;
