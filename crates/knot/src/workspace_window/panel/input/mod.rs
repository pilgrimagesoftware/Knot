//! The panel's composer: the prompt entry and everything stacked around
//! it, one module per row.
//!
//! - [`area`] stacks the rows and owns the drop target around them.
//! - [`queued`] draws the prompts waiting to be sent.
//! - [`chips`] draws the attached-context strip.
//! - [`entry`] draws the row the user types into.
//! - [`controls`] draws the control bar and the session-config selectors.
//! - [`config_select`] backs the two selectors that scroll and search, and
//!   names which ones those are.

mod area;
mod chips;
mod config_select;
mod controls;
mod entry;
mod queued;

pub(crate) use config_select::ConfigSelectorDelegate;
pub(crate) use config_select::ConfigSelectorItem;
pub(in crate::workspace_window) use config_select::EFFORT_SELECTOR_ID;
#[cfg(test)]
pub(crate) use config_select::MENU_MAX_HEIGHT_REMS;
pub(in crate::workspace_window) use config_select::MODEL_SELECTOR_ID;
pub(in crate::workspace_window) use config_select::SEARCHABLE_SELECTORS;
pub(in crate::workspace_window) use config_select::SelectorKey;
#[cfg(test)]
pub(crate) use config_select::config_selector_items;
#[cfg(test)]
pub(crate) use config_select::new_config_select_state;

/// The permission-mode selector's element id - the one selector with
/// risk-tinted labels and a keyboard action of its own, so it needs naming
/// rather than matching on a literal in three places.
pub(in crate::workspace_window) const PERMISSION_SELECTOR_ID: &str =
    "panel-permission-mode-selector";

#[cfg(test)]
mod tests;
