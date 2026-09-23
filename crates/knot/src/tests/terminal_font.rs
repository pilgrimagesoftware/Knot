//! The terminal's font family is resolved from the installed families once
//! per configured name, not once per frame.
//!
//! Contract: `openspec/specs/terminal-rendering/spec.md`, "Drawing a terminal
//! frame does not enumerate installed fonts". The defect this pins cost
//! ~11.4ms per call on macOS and ran twice a frame, on a pane that repaints
//! for every keystroke it echoes.
//!
//! Asserted as a call count rather than a duration: a timing assertion would
//! measure the machine rather than the code, and the project keeps no flaky
//! tests.

use std::cell::Cell;

use crate::workspace_window::terminal_font::TerminalFont;
use crate::workspace_window::terminal_font::resolve;

/// Standing in for `cx.text_system().all_font_names()` - the same shape
/// `tests::settings_font_preview` uses for the settings window's half of
/// this question.
fn installed() -> Vec<String> {
    vec!["Helvetica Neue".to_string(), "JetBrains Mono".to_string()]
}

/// A stand-in that counts how many times it was asked, so a test can assert
/// what a run of frames costs.
struct Enumerations(Cell<usize>);

impl Enumerations {
    fn new() -> Self {
        Self(Cell::new(0))
    }

    fn families(&self) -> Vec<String> {
        self.0.set(self.0.get() + 1);
        installed()
    }

    fn count(&self) -> usize {
        self.0.get()
    }
}

#[test]
fn an_installed_family_is_used_as_configured() {
    assert_eq!(resolve("Helvetica Neue", &installed()), "Helvetica Neue");
}

#[test]
fn an_uninstalled_family_falls_back_to_the_embedded_default() {
    assert_eq!(resolve("Not Installed", &installed()), "JetBrains Mono");
}

/// An empty `terminal_font_name` is not a family the text system can
/// resolve, so it takes the same route as an uninstalled one rather than
/// reaching the proportional UI font.
#[test]
fn an_empty_family_falls_back_too() {
    assert_eq!(resolve("", &installed()), "JetBrains Mono");
}

/// The regression test for this change: a run of frames must not pay for the
/// enumeration.
#[test]
fn resolving_the_same_family_enumerates_once() {
    let mut memo = TerminalFont::default();
    let enumerations = Enumerations::new();

    for _ in 0..100 {
        assert_eq!(memo.family("Helvetica Neue", || enumerations.families()),
                   "Helvetica Neue");
    }

    assert_eq!(enumerations.count(),
               1,
               "a hundred frames must enumerate the installed families once");
}

/// The other half of the memo's contract: it is keyed on the name that was
/// asked for, so a settings change is picked up without a separate
/// invalidation hook for a later caller to remember.
#[test]
fn a_changed_family_is_resolved_again() {
    let mut memo = TerminalFont::default();
    let enumerations = Enumerations::new();

    assert_eq!(memo.family("Helvetica Neue", || enumerations.families()),
               "Helvetica Neue");
    assert_eq!(memo.family("JetBrains Mono", || enumerations.families()),
               "JetBrains Mono");

    assert_eq!(enumerations.count(), 2);
}

/// A fallback is remembered like any other answer, so an unresolvable name is
/// not re-checked every frame. That is the case the defect made most
/// expensive: a stale persisted name matches nothing, so the scan ran to the
/// end of the list every time.
#[test]
fn a_fallback_is_remembered_rather_than_re_checked() {
    let mut memo = TerminalFont::default();
    let enumerations = Enumerations::new();

    for _ in 0..100 {
        assert_eq!(memo.family("Not Installed", || enumerations.families()),
                   "JetBrains Mono");
    }

    assert_eq!(enumerations.count(), 1);
}
