//! The sidebar's compact breakpoint: where it falls, and that the four
//! surfaces reading it read one predicate rather than four literals.

use knot_core::consts::{
    SIDEBAR_COMPACT_BREAKPOINT, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};

use super::*;

#[test]
fn a_sidebar_narrower_than_the_breakpoint_is_compact() {
    assert!(sidebar_is_compact(SIDEBAR_COMPACT_BREAKPOINT - 1.0));
    assert!(sidebar_is_compact(SIDEBAR_WIDTH_MIN));
}

/// The breakpoint is the narrowest width that still fits the full row, so
/// the boundary itself is not compact.
#[test]
fn a_sidebar_exactly_at_the_breakpoint_is_not_compact() {
    assert!(!sidebar_is_compact(SIDEBAR_COMPACT_BREAKPOINT));
}

#[test]
fn a_sidebar_wider_than_the_breakpoint_is_not_compact() {
    assert!(!sidebar_is_compact(SIDEBAR_COMPACT_BREAKPOINT + 1.0));
    assert!(!sidebar_is_compact(SIDEBAR_WIDTH_DEFAULT));
    assert!(!sidebar_is_compact(SIDEBAR_WIDTH_MAX));
}

/// The breakpoint has to sit inside the range the divider can reach, or one
/// of the two layouts would be unreachable.
#[test]
fn the_breakpoint_lies_between_the_bounds() {
    const { assert!(SIDEBAR_WIDTH_MIN < SIDEBAR_COMPACT_BREAKPOINT) };
    const { assert!(SIDEBAR_COMPACT_BREAKPOINT < SIDEBAR_WIDTH_MAX) };
    assert!((SIDEBAR_WIDTH_MIN..=SIDEBAR_WIDTH_MAX).contains(&SIDEBAR_WIDTH_DEFAULT));
}

/// The compact new-agent button has no label, so its tooltip is the only
/// thing naming it - a missing key would ship the key string as the name.
#[test]
fn the_new_agent_label_resolves() {
    let key = "sidebar.new_agent";
    assert_ne!(knot_core::l10n::t(key),
               key,
               "{key} is missing from the catalog");
}
