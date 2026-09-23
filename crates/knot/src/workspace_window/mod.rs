//! The workspace window - one window per workspace, holding that workspace's
//! agents, their terminal or panel sessions, and the views over them.
//!
//! `window` holds the entity itself; every other module here hangs an `impl`
//! off it for one concern - `sessions` for the PTYs, `panel` for the ACP side,
//! `render` for the element tree, `repaint` for the poll that drives it.

mod agent_row;
mod agents;
mod chrome;
pub(crate) mod composer_focus;
mod creation;
mod git_panel;
pub(crate) mod mcp_panel;
mod menus;
mod notifications;
mod open;
pub(crate) mod panel;
mod process_actions;
mod processes;
pub(crate) mod prompt_queue;
mod pull_requests;
mod pull_requests_view;
mod render;
mod repaint;
mod sessions;
mod settings_refresh;
mod sidebar_layout;
mod terminal_input;
mod title;
mod view_mode;
mod window;

// Re-exported so the rest of the crate keeps reaching these by
// `workspace_window::<name>`, as it did when they lived here.
pub(crate) use chrome::*;
pub(crate) use menus::*;
pub(crate) use sidebar_layout::sidebar_is_compact;
pub(crate) use title::workspace_title;
pub(crate) use view_mode::WorkspaceViewMode;
pub(crate) use window::WorkspaceWindow;
