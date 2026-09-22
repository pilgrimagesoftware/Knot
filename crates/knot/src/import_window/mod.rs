//! The Import window - bringing personas and workspaces in from tools the
//! user already has (`openspec/specs/import-ui`).
//!
//! [`window`] holds the state and how the window opens; [`pane`] holds what it
//! draws.

mod outcome;
mod pane;
mod window;

/// Public within the crate because `tests::import_window` asserts its row
/// labels directly; the window itself is only ever opened through the action.
#[cfg(test)]
pub(crate) use window::ImportWindow;
pub(crate) use window::register_import_action;
