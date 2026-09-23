use std::time::Duration;

use knot_core::ViewMode;

use super::{section_is_shown, taken_ago_text};

/// Panel mode only. A Terminal agent has a real PTY, so its own `/mcp` works
/// there and a second way to ask would be two answers to one question.
#[test]
fn the_section_is_panel_mode_only() {
    assert!(section_is_shown(ViewMode::Panel, false, false));
    assert!(!section_is_shown(ViewMode::Terminal, false, false));
}

/// A takeover or a document pane is not the agent's session, so the section
/// does not belong under either.
#[test]
fn the_section_is_absent_over_a_takeover_or_a_document() {
    assert!(!section_is_shown(ViewMode::Panel, true, false));
    assert!(!section_is_shown(ViewMode::Panel, false, true));
    assert!(!section_is_shown(ViewMode::Panel, true, true));
}

#[test]
fn every_age_shape_resolves_from_the_catalog() {
    for elapsed in [Duration::from_secs(0),
                    Duration::from_secs(4),
                    Duration::from_secs(30),
                    Duration::from_secs(90),
                    Duration::from_secs(7_200)]
    {
        let text = taken_ago_text(elapsed);

        assert!(!text.starts_with("mcp."), "unresolved catalog key: {text}");
        assert!(!text.is_empty());
    }
}

/// A probe that just landed should not read as "checked 0s ago" - that
/// phrasing makes a fresh answer look like a stale one.
#[test]
fn a_fresh_probe_reads_as_just_now() {
    let just_now = taken_ago_text(Duration::from_secs(0));

    assert_eq!(taken_ago_text(Duration::from_secs(4)), just_now);
    assert_ne!(taken_ago_text(Duration::from_secs(30)), just_now);
}

/// The number has to survive substitution, or the row says "checked  ago".
#[test]
fn the_age_carries_its_number() {
    assert!(taken_ago_text(Duration::from_secs(42)).contains("42"));
    assert!(taken_ago_text(Duration::from_secs(300)).contains('5'),
            "five minutes");
    assert!(taken_ago_text(Duration::from_secs(10_800)).contains('3'),
            "three hours");
}
