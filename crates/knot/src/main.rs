//! The `knot` binary.
//!
//! NOTE: no crate-wide `allow(dead_code)`. Anything unreachable carries its
//! own `#[allow(dead_code)]` and a comment saying why - grep `UNWIRED` for
//! the ported-but-not-yet-connected inventory, and `SUPERSEDED` for code a
//! newer path replaced.

mod about_window;
mod agent_editor;
mod agent_menu;
mod agent_processes;
mod app_bootstrap;
mod app_state;
mod app_support;
mod appearance;
mod broadcast_sheet;
mod command_center;
mod commit_window;
// UNWIRED(#396): the composer's scanner. It decides where the styled runs
// are; the decoration collections that ask it are the next step of the
// rich-prompt-composer change, and nothing outside its own tests calls it
// until they land. The allow comes off in the same commit they do.
//
// Its tests are therefore not coverage of anything the user can reach yet,
// which `composer_scan/tests/mod.rs` says too.
//
// `unused_imports` rides along because the module's re-exports are its
// intended surface, and an unused re-export is the same absent caller the
// `dead_code` allow is about.
#[allow(dead_code, unused_imports)]
mod composer_scan;
mod consts;
mod controls;
mod dashboard;
mod diff_stats;
mod external_tools;
mod git_panel;
mod import_window;
mod macos;
mod markdown_view;
mod mcp_status;
mod open_in;
mod panel_commands;
mod panel_session;
mod panel_state;
mod panel_view;
mod plan_view;
mod pull_request_state;
mod quit_guard;
mod refresh_cache;
mod settings_broadcast;
mod settings_window;
mod terminal_view;
#[cfg(test)]
mod tests;
mod window_options;
mod window_registry;
mod working_indicator;
mod workspace_manager;
mod workspace_window;

fn main() {
    // Before anything builds a git runner: a Finder-launched app's PATH
    // names none of the places git may actually live.
    external_tools::configure();
    app_bootstrap::run();
}
