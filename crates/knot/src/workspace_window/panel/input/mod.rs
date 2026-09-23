//! The panel's composer: the prompt entry and everything stacked around
//! it, one module per row.
//!
//! - [`area`] stacks the rows and owns the drop target around them.
//! - [`queued`] draws the prompts waiting to be sent.
//! - [`chips`] draws the attached-context strip.
//! - [`entry`] draws the row the user types into.
//! - [`controls`] draws the control bar and the session-config selectors.

mod area;
mod chips;
mod controls;
mod entry;
mod queued;

/// The permission-mode selector's element id - the one selector with
/// risk-tinted labels and a keyboard action of its own, so it needs naming
/// rather than matching on a literal in three places.
pub(in crate::workspace_window) const PERMISSION_SELECTOR_ID: &str =
    "panel-permission-mode-selector";
