//! Panel-mode: the ACP-backed conversation view, split by what each part
//! is responsible for.
//!
//! - [`session`] starts, stops and probes the ACP connection.
//! - [`pane`] draws the conversation and the panes that replace it.
//! - [`input`] draws the composer.
//! - [`composer`] builds the composer entity and owns its send chord.
//! - [`lookup`] completes slash commands and skills inside the composer.
//! - [`prompt`] moves text from the composer to the agent.
//! - [`styling`] keeps the composer's styled runs in step with it.
//! - [`mentions`] lists the agent's files for the `@` lookup.
//! - [`attachments`] keeps a chip in the buffer in step with its row.
//! - [`shell`] runs a `!` command and hands its result to the next prompt.
//!
//! Every item here is an inherent method on
//! [`super::WorkspaceWindow`]; the modules carve the impl up by concern,
//! not by type.

mod attachments;
pub(crate) mod composer;
pub(crate) mod input;
pub(crate) mod lookup;
pub(crate) mod mentions;
mod pane;
pub(crate) mod prompt;
mod session;
pub(super) mod shell;
mod styling;
