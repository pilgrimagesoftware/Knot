//! The `knot` binary.
//!
//! NOTE: no crate-wide `allow(dead_code)`. Anything unreachable carries its
//! own `#[allow(dead_code)]` and a comment saying why - grep `UNWIRED` for
//! the ported-but-not-yet-connected inventory, and `SUPERSEDED` for code a
//! newer path replaced.

mod about_window;
mod agent_editor;
mod agent_menu;
mod app_bootstrap;
mod app_state;
mod app_support;
mod broadcast_sheet;
mod command_center;
mod consts;
mod controls;
mod dashboard;
mod diff_stats;
mod import_window;
mod macos;
mod markdown_view;
mod open_in;
mod panel_commands;
mod panel_session;
mod panel_state;
mod panel_view;
mod plan_view;
mod quit_guard;
mod settings_window;
mod terminal_view;
#[cfg(test)]
mod tests;
mod window_options;
mod working_indicator;
mod workspace_manager;
mod workspace_window;

fn main() {
    app_bootstrap::run();
}
