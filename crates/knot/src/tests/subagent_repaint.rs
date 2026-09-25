//! The chain that gets a subagent onto a frame, asserted over the sources.
//!
//! Every link in this chain has been broken at least once elsewhere in the
//! workspace - `.claude/rules/rust-structure.md` names four, none of which
//! failed a test. The symptom is an absence, and an absence has no output
//! channel, which is why these are grep tests over the code rather than
//! behavioural ones: what needs proving is that a call site *exists* and that
//! no second one does.

use std::path::Path;

fn src() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(src().join("src").join(relative)).expect("read a source file")
}

fn visit_rust_files(path: &Path, found: &mut impl FnMut(&Path)) {
    if path.is_file() {
        found(path);
        return;
    }

    let Ok(entries) = std::fs::read_dir(path)
    else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() || path.extension().is_some_and(|extension| extension == "rs") {
            visit_rust_files(&path, found);
        }
    }
}

/// A flag nobody polls is indistinguishable from one that works until the
/// window goes quiet - `pull_request_states` shipped in exactly that state.
#[test]
fn the_repaint_poll_takes_the_subagent_changed_flag() {
    let repaint = read("workspace_window/repaint.rs");

    assert!(repaint.contains("self.subagents.lock().take_changed()"),
            "repaint_poll_tick must drain the subagent registry's changed flag");
    assert!(repaint.contains("|| subagents_changed"),
            "the drained flag must contribute to the notify condition");
}

/// The consuming read must have exactly one caller. A second reader would
/// take the change the first one needed, and the frame would never be asked
/// for - the `panel_needs_repaint` defect, which consumed `take_changed` and
/// then hit an early return that discarded it.
#[test]
fn only_the_repaint_poll_takes_the_subagent_changed_flag() {
    let mut callers = Vec::new();

    visit_rust_files(&src().join("src"), &mut |file| {
        let text = std::fs::read_to_string(file).expect("read a source file");
        if text.contains("subagents.lock().take_changed()")
           || text.contains("subagents().take_changed()")
        {
            callers.push(file.display().to_string());
        }
    });

    // This test's own source mentions the call in a string literal.
    callers.retain(|path| !path.ends_with("subagent_repaint.rs"));

    assert_eq!(callers.len(),
               1,
               "exactly one caller may consume the flag, found: {callers:?}");
    assert!(callers[0].ends_with("repaint.rs"),
            "the caller must be the repaint poll: {callers:?}");
}

/// Assigned to a local before the `if`, never inlined into it. `||`
/// short-circuits, and because the read is a consuming swap a skipped flag
/// stays set and fires a spurious repaint on the next tick.
#[test]
fn the_flag_is_read_into_a_local_rather_than_inlined_into_the_condition() {
    let repaint = read("workspace_window/repaint.rs");

    assert!(repaint.contains("let subagents_changed = self.subagents.lock().take_changed();"),
            "the consuming read must be assigned before the condition combines it");
}

/// The spec bars *parsing* on the render path, not reading: "a render SHALL
/// read only what has already been recorded". So the recognizer and the
/// feed's entry points are what may not appear under a render module - the
/// registry read that draws a row is the intended shape.
#[test]
fn no_render_module_recognizes_a_subagent() {
    const RECOGNITION_CALLS: &[&str] = &["recognize::for_agent_type",
                                         "for_agent_type(",
                                         "recognize_hook(",
                                         ".recognize(",
                                         "SubagentSink"];

    let render_paths = ["workspace_window/render", "panel_view", "terminal_view.rs"];

    let mut offenders = Vec::new();
    for path in render_paths {
        visit_rust_files(&src().join("src").join(path), &mut |file| {
            let text = std::fs::read_to_string(file).expect("read a render source");
            for call in RECOGNITION_CALLS {
                if text.contains(call) {
                    offenders.push(format!("{}: {call}", file.display()));
                }
            }
        });
    }

    assert!(offenders.is_empty(),
            "render code must read recorded subagents, never recognize one: {offenders:?}");
}

/// Recording must not be gated on the pane being shown. A subagent
/// dispatched while the user was looking at another agent is exactly the one
/// worth having on return, which is what separates this from the process
/// sampler - that one *is* gated on being shown, deliberately.
#[test]
fn the_acp_feed_is_not_gated_on_a_pane_being_shown() {
    let feed = read("subagent_feed.rs");

    for gate in ["selected_agent", "is_shown", "section_is_shown", "showing"] {
        assert!(!feed.contains(gate),
                "the subagent feed must not consult {gate}: recording is independent of what is \
                 drawn");
    }
}

/// Teardown clears the registry. Unlike the window's other per-agent maps
/// this one is shared with the MCP hook route and outlives the window, so an
/// entry left behind leaks past the window's own life.
#[test]
fn tearing_down_a_session_clears_its_subagents() {
    let sessions = read("workspace_window/sessions.rs");

    assert!(sessions.contains("self.subagents.lock().clear(id)"),
            "teardown_session must clear the agent's subagent records");
}
