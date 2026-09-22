//! Unit tests for [`super`].

use super::*;

/// An id typed twice is two types that behave as one, which is exactly the
/// confusion the roster exists to prevent.
#[test]
fn every_id_appears_once() {
    let mut ids = ALL.iter()
                     .map(|agent_type| agent_type.id)
                     .collect::<Vec<_>>();
    ids.sort_unstable();
    let unique = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), unique);
}

#[test]
fn the_default_and_shell_ids_are_in_the_roster() {
    assert!(info(DEFAULT).is_some());
    assert!(is_shell(SHELL));
}

/// Exactly one type is a shell: the view mode, the launch path and the
/// tracking preset all branch on it, so a second one would have to be
/// taught to each of them separately.
#[test]
fn only_one_type_is_a_shell() {
    assert_eq!(ALL.iter().filter(|agent_type| agent_type.is_shell).count(),
               1);
}

/// A custom type is a user's own command, so nothing may assume vendor
/// behaviour of it - no inline registration, no hook-driven activity.
#[test]
fn a_custom_type_claims_no_vendor_behaviour() {
    for agent_type in ALL.iter().filter(|agent_type| agent_type.is_custom) {
        assert!(!agent_type.inline_registration,
                "{} claims inline registration",
                agent_type.id);
        assert!(!agent_type.hook_activity,
                "{} claims hook activity",
                agent_type.id);
    }
}

/// An unrecognized type is a working configuration - it launches through
/// the terminal path - so every lookup has to answer for one rather than
/// panicking or pretending it is the default.
#[test]
fn an_unknown_type_answers_as_itself() {
    assert_eq!(info("nothing-by-that-name"), None);
    assert_eq!(label("nothing-by-that-name"), "nothing-by-that-name");
    assert!(!is_shell("nothing-by-that-name"));
    assert!(!supports_inline_registration("nothing-by-that-name"));
    assert!(!has_hook_activity("nothing-by-that-name"));
}

#[test]
fn a_known_type_answers_from_its_row() {
    assert_eq!(label("opencode"), "OpenCode");
    assert!(supports_inline_registration("gemini"));
    assert!(has_hook_activity("claude"));
    assert!(!has_hook_activity("gemini"));
}
