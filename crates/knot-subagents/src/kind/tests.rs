use super::SubagentKind;

#[test]
fn a_missing_kind_is_unstated() {
    assert_eq!(SubagentKind::from_reported(None), SubagentKind::Unstated);
}

#[test]
fn an_empty_kind_is_unstated_rather_than_a_named_empty_string() {
    assert_eq!(SubagentKind::from_reported(Some("")),
               SubagentKind::Unstated);
    assert_eq!(SubagentKind::from_reported(Some("   ")),
               SubagentKind::Unstated);
}

/// The scenario this type exists for. An agent that genuinely configures a
/// persona called `unstated` must not be indistinguishable from one that named
/// no persona at all - which is exactly what a sentinel string would make it.
#[test]
fn a_kind_literally_named_unstated_is_not_the_absent_one() {
    let named = SubagentKind::from_reported(Some("unstated"));

    assert_ne!(named, SubagentKind::Unstated);
    assert_eq!(named.name(), Some("unstated"));
    assert!(named.is_stated());
}

#[test]
fn an_unstated_kind_has_no_name_for_the_caller_to_render() {
    let kind = SubagentKind::Unstated;

    assert_eq!(kind.name(), None);
    assert!(!kind.is_stated());
}

#[test]
fn a_named_kind_keeps_the_agents_own_spelling() {
    let kind = SubagentKind::from_reported(Some("Code-Review"));

    assert_eq!(kind.name(), Some("Code-Review"));
}

#[test]
fn surrounding_whitespace_is_trimmed_so_it_never_reaches_the_header() {
    let kind = SubagentKind::from_reported(Some("  discovery\n"));

    assert_eq!(kind, SubagentKind::Named("discovery".to_owned()));
}

/// `Display` must not invent English for the absent case - that is the
/// catalogue's job, and a placeholder here would be the easiest route for an
/// untranslated word to reach the screen.
#[test]
fn display_writes_the_name_and_nothing_for_the_absent_case() {
    assert_eq!(SubagentKind::Named("discovery".to_owned()).to_string(),
               "discovery");
    assert_eq!(SubagentKind::Unstated.to_string(), "");
}
