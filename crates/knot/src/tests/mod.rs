//! Unit tests for the `knot` binary, one module per subject.
//!
//! The shared `workspace` fixture stays here; anything used by a single
//! subject lives with it.

use uuid::Uuid;

use crate::app_state::AgentMenuFacts;
use crate::app_state::agent_context_menu_entries;

mod about_window;
mod agent_context_menu;
mod agent_signals;
mod agents_menu;
mod l10n_catalog;
mod layout_model;
mod markdown_view;
mod notifications;
mod quit_warning;
mod settings_font_preview;
mod settings_labels;
mod sidebar_menu;
mod sidebar_width;
mod startup;
mod workspace_dialog;
mod workspace_window_config;

use knot_core::Workspace;

fn workspace(name: &str) -> Workspace {
    Workspace { id:                    Uuid::new_v4(),
                name:                  name.to_string(),
                color_hex:             "#123456".to_string(),
                agent_ids:             Vec::new(),
                layout_mode:           "single".to_string(),
                active_agent_ids:      Vec::new(),
                focused_pane_index:    0,
                split_ratio:           0.5,
                split_ratio_secondary: None,
                show_dashboard:        None,
                is_detached:           None,
                window_bounds:         None, }
}

/// The labels a given set of facts produces, separators rendered as
/// `"-"` so ordering *and* divider placement are both asserted.
fn menu_labels(facts: AgentMenuFacts) -> Vec<&'static str> {
    agent_context_menu_entries(facts).into_iter()
                                     .map(|entry| entry.label().unwrap_or("-"))
                                     .collect()
}
