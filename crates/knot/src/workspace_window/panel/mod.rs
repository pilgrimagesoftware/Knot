//! Panel-mode: the ACP-backed conversation view, split by what each part
//! is responsible for.
//!
//! - [`session`] starts, stops and probes the ACP connection.
//! - [`pane`] draws the conversation and the panes that replace it.
//! - [`input`] draws the composer.
//! - [`lookup`] completes slash commands and skills inside the composer.
//! - [`prompt`] moves text from the composer to the agent.
//!
//! Every item here is an inherent method on
//! [`super::WorkspaceWindow`]; the modules carve the impl up by concern,
//! not by type.

pub(super) mod input;
pub(crate) mod lookup;
mod pane;
mod prompt;
mod session;
