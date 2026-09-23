//! The gate, and two source-level guards for the chain the probe depends on.
//!
//! The guards read the crate's own source. That is unusual, and it is here
//! because both rules they encode - no I/O on the render path, and an
//! off-thread result must reach a frame - have broken repeatedly without ever
//! failing a test. `.claude/rules/rust-structure.md` names four separate
//! breaks of the second one, none of which any test noticed. A grep is a poor
//! check, but it is a check, and an absence has no other output channel.

use std::fs;
use std::path::{Path, PathBuf};

use knot_core::ViewMode;

use super::probes;

fn workspace_window_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/workspace_window")
}

fn read(relative: &str) -> String {
    let path = workspace_window_dir().join(relative);

    fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Every non-test `.rs` file under a directory, recursively.
///
/// Test sources are excluded because these guards quote the very identifiers
/// they forbid - this file names `mcp_results.lock()` in an assertion, and
/// without the filter the guard reports itself as a second drain.
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

#[test]
fn a_shown_running_panel_agent_is_probed() {
    assert!(probes(Some(ViewMode::Panel), Some(1717)));
}

/// Their own `/mcp` already works: a Terminal agent has a real PTY to type it
/// into. Probing one would run a second CLI to answer a question the user can
/// already ask directly.
#[test]
fn a_terminal_agent_is_never_probed() {
    assert!(!probes(Some(ViewMode::Terminal), Some(4242)));
}

/// No session root means never started, deactivated, mid-restart or exited -
/// `agent-processes`' own definition of not running. There is nothing to ask
/// about and the CLI would answer for whatever configuration happens to be on
/// disk rather than for a live agent.
#[test]
fn an_agent_with_no_live_session_is_not_probed() {
    assert!(!probes(Some(ViewMode::Panel), None));
    assert!(!probes(Some(ViewMode::Terminal), None));
}

/// An agent the store does not know cannot be probed either - there is no
/// view mode to decide on.
#[test]
fn an_unknown_agent_is_not_probed() {
    assert!(!probes(None, Some(1717)));
    assert!(!probes(None, None));
}

/// The repaint chain. A result that lands off the main thread reaches the
/// screen only if this tick is called and its answer is in the `if`, and
/// both halves have been broken separately before.
#[test]
fn the_probe_tick_is_called_from_the_repaint_poll_and_its_answer_is_used() {
    let repaint = read("repaint.rs");

    assert!(repaint.contains("self.mcp_probe_tick()"),
            "repaint_poll_tick must call the probe tick, or results never reach a frame");
    assert!(repaint.contains("|| mcp_probed"),
            "the tick's answer must be in the notify chain, or a landed probe draws only when \
             something unrelated happens to repaint");
}

/// The clearing read must live in exactly one place. A second drain consumes
/// a result this one then reports as absent - the "flag read and then
/// discarded" shape that cost a release once already.
#[test]
fn the_results_queue_is_drained_in_exactly_one_place() {
    let drains: Vec<_> = sources_under(&workspace_window_dir()).into_iter()
                                                               .filter(|path| {
                                                                   fs::read_to_string(path)
                    .is_ok_and(|text| text.contains("mcp_results.lock()"))
                                                               })
                                                               .collect();

    assert_eq!(drains.len(),
               1,
               "`mcp_results` must be taken in one place only; found {drains:?}");
    assert!(drains[0].ends_with("mcp_panel/probe.rs"),
            "found the drain in {drains:?}");
}

/// No I/O on the render path. A probe runs an agent's CLI, which
/// health-checks every configured server over the network; on the render path
/// that runs per keystroke.
#[test]
fn nothing_under_render_starts_a_probe() {
    let render_dir = workspace_window_dir().join("render");

    for path in sources_under(&render_dir) {
        let text = fs::read_to_string(&path).expect("a render source");

        for forbidden in ["mcp_probe_tick", "knot_mcp_probe::probe", "spawn_blocking"] {
            assert!(!text.contains(forbidden),
                    "{} names {forbidden}: the render path must not start a probe",
                    path.display());
        }
    }
}
