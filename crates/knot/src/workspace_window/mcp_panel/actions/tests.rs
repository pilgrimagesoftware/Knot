//! The parts of the handover that do not need a window.
//!
//! Creating a companion and starting its session needs a live
//! `WorkspaceWindow`, so what is checked here is the decision that precedes
//! it: which binary the flow opens in, and that the shell companion path is
//! what carries it rather than the ACP adapter.

use std::fs;
use std::path::{Path, PathBuf};

/// The source of the module this file tests.
fn actions_source() -> String {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/workspace_window/mcp_panel/actions.rs");

    fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

fn sources_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();

    let Ok(entries) = fs::read_dir(dir)
    else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(sources_under(&path));
        }
        else if path.extension().is_some_and(|ext| ext == "rs")
                  && path.file_name().is_some_and(|name| name != "tests.rs")
        {
            found.push(path);
        }
    }

    found
}

/// The terminal is a shell companion - the path `acp-panel-ui` already
/// guarantees - and never the agent's own session.
#[test]
fn the_handover_opens_a_shell_companion() {
    let source = actions_source();

    assert!(source.contains("create_shell_companion"),
            "the handover must use the plain-shell path");
    assert!(source.contains("shell_command"),
            "the flow is entered through the companion's initialization command");
}

/// The ACP adapter speaks JSON-RPC, not shell. Typing at it would corrupt the
/// very session the user is trying to repair, so nothing here may reach it.
#[test]
fn nothing_in_the_handover_touches_the_acp_session() {
    let source = actions_source();

    for forbidden in ["panel_sessions",
                      "deliver_panel_prompt",
                      "adapter_root",
                      "send_text"]
    {
        assert!(!source.contains(forbidden),
                "the handover names {forbidden}: it must not reach the agent's own session");
    }
}

/// The re-probe on exit has to be wired into the poll's exit loop, or a
/// terminal the user finished in leaves the section showing what it knew
/// before they started.
#[test]
fn the_exit_hook_is_called_from_the_repaint_poll() {
    let repaint = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/workspace_window/repaint.rs");
    let source = fs::read_to_string(repaint).expect("the repaint poll");

    assert!(source.contains("finish_mcp_handover"),
            "a delegated terminal's exit must re-probe the section that opened it");
}

/// One place decides which binary the flow opens in. Two would drift, and the
/// drift is invisible: the probe would read one installation's configuration
/// while the handover opened another's.
#[test]
fn the_program_is_resolved_in_one_place() {
    let window_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/workspace_window");

    let resolvers: Vec<_> =
        sources_under(&window_dir).into_iter()
                                  .filter(|path| {
                                      fs::read_to_string(path).is_ok_and(|text| {
                                                                  text.contains("agent_commands")
                                                              })
                                  })
                                  .collect();

    assert_eq!(resolvers.len(),
               1,
               "`agent_commands` should be read in one place; found {resolvers:?}");
    assert!(resolvers[0].ends_with("mcp_panel/actions.rs"),
            "found it in {resolvers:?}");
}
