//! That the seam produces a usable window.
//!
//! Every one of the window's per-frame memos and per-agent maps was verified
//! by reading rather than by a test, because reaching one meant having a
//! `WorkspaceWindow` and the only way to get one was `open` - which raises an
//! OS window, registers it, spawns a repaint poll and starts a session per
//! agent. What stood in for a test was a source guard asserting that a call
//! appears in the frame's preamble, which pins that a line exists rather than
//! that it does anything.
//!
//! These cover the seam itself: that it builds, that what it builds shares
//! the process's settings surface rather than a snapshot of it, and that it
//! starts nothing. The cases it exists to make testable belong with the code
//! they are about, not here.

use tempfile::TempDir;

use super::open_test_window;
use crate::workspace_window::terminal_font::DEFAULT_FAMILY;

/// A settings surface rooted in `dir`, so nothing here can reach the
/// developer's own workspaces and agents.
fn settings(dir: &TempDir) -> knot_core::Settings {
    knot_core::Settings::with_store_root(dir.path())
}

/// The window builds, and a memo run against it answers.
///
/// The answer is the fallback whatever `terminal_font_name` says, and that is
/// not an accident of this test: `cx.text_system().all_font_names()` is empty
/// under `gpui_kit::test`, so no configured name resolves and
/// `terminal_font::resolve` returns [`DEFAULT_FAMILY`] every time. That makes
/// this deterministic rather than dependent on what is installed on the
/// machine running it - and it pins the fallback path through the window,
/// which is the one that matters: an unresolvable name left to GPUI's own
/// lookup lands on the proportional UI font, and a terminal grid in a
/// proportional face is unreadable rather than merely wrong.
#[gpui_kit::test]
fn the_seam_builds_a_window_whose_memos_run(cx: &mut gpui_kit::TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let mut chosen = settings(&dir);
    chosen.terminal_font_name = "A Font That Is Not Installed".to_string();
    let (mut cx, view, _store) = open_test_window(cx, chosen);

    let family = view.update(&mut cx, |view, cx| {
                         view.refresh_terminal_font(cx);
                         view.terminal_font_family()
                     });

    assert_eq!(family.as_ref(),
               DEFAULT_FAMILY,
               "a name the text system cannot resolve has to fall back to the embedded family, \
                and the window has to be the thing that does it");
}

/// What the seam builds is on the process's settings surface, not holding a
/// copy of it.
///
/// This is the property #238 was about and the one every later defect in this
/// area has turned on: a window that reads a preference once and holds it is
/// indistinguishable from one that reads live until the setting moves
/// underneath it. A test can only tell those apart if it can hold a window
/// across the write, which is what the seam is for - so it is worth pinning
/// that the seam does not quietly hand back a window with a surface of its
/// own.
#[gpui_kit::test]
fn the_seam_shares_the_process_settings_surface(cx: &mut gpui_kit::TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let mut chosen = settings(&dir);
    chosen.agent_panel_shift_enter_sends = false;
    let (mut cx, view, _store) = open_test_window(cx, chosen);

    let (before, after) =
        view.update(&mut cx, |_view, cx| {
                let before = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
                crate::settings_global::write(cx, |settings| {
                    settings.agent_panel_shift_enter_sends = true
                });
                let after = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
                (before, after)
            });

    assert!(!before,
            "the window starts on the setting it was built with");
    assert!(after,
            "and a write made while it is open has to be what it reads next - a window holding \
             its own copy would still answer false here");
}

/// The seam starts nothing.
///
/// Session startup stays in `open` rather than moving into the constructor
/// for exactly this reason: `ensure_session` spawns a PTY and
/// `ensure_panel_session` an adapter subprocess, per agent, so a unit test
/// that went through the whole of `open` would leave processes behind on
/// every run.
#[gpui_kit::test]
fn the_seam_starts_nothing(cx: &mut gpui_kit::TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let (mut cx, view, _store) = open_test_window(cx, settings(&dir));

    view.update(&mut cx, |view, _cx| {
            assert!(view.sessions.is_empty(),
                    "the seam must not start a terminal session");
            assert!(view.panel_sessions.is_empty(), "nor a panel session");
            assert!(view.selected_agent.is_none(),
                    "and it selects nothing, having no agents");
        });
}
