//! Tests for the shared import result and how it renders.
//!
//! Per the catalogue-key contract these assert that the copy *resolves* and
//! that the values the renderer must not lose - the counts and every name -
//! survive into the rendered line. They do not assert the English wording,
//! which is a copy edit away from changing.

use super::{ImportResult, Unreadable, UnreadableReason};
use crate::l10n::t;

fn full_result() -> ImportResult {
    ImportResult { added:      vec!["architect-reviewer".into(), "debugger".into()],
                   skipped:    vec!["code-reviewer".into()],
                   unreadable: vec![Unreadable::new("broken.md", UnreadableReason::NoName)], }
}

#[test]
fn an_untouched_result_is_empty() {
    assert!(ImportResult::default().is_empty());
}

#[test]
fn a_result_with_only_skips_is_not_empty() {
    let result = ImportResult { skipped: vec!["one".into()],
                                ..ImportResult::default() };

    assert!(!result.is_empty());
}

/// Task 1.2: a result holding all three kinds renders each of them.
#[test]
fn all_three_kinds_render() {
    let lines = full_result().summary_lines();

    assert_eq!(lines.len(), 3, "one line per non-empty category");
}

#[test]
fn every_name_survives_into_the_rendered_lines() {
    let rendered = full_result().summary_lines().join("\n");

    for name in ["architect-reviewer",
                 "debugger",
                 "code-reviewer",
                 "broken.md"]
    {
        assert!(rendered.contains(name),
                "{name} should be named in the summary");
    }
}

#[test]
fn each_line_carries_its_count() {
    let lines = full_result().summary_lines();

    assert!(lines[0].contains('2'), "two were added");
    assert!(lines[1].contains('1'), "one was skipped");
    assert!(lines[2].contains('1'), "one was unreadable");
}

/// An unreadable record reports *why*, not just that it failed.
#[test]
fn an_unreadable_record_renders_its_reason() {
    let rendered = full_result().summary_lines().join("\n");

    assert!(rendered.contains(&t(UnreadableReason::NoName.l10n_key())));
}

#[test]
fn an_empty_category_contributes_no_line() {
    let result = ImportResult { added: vec!["only-one".into()],
                                ..ImportResult::default() };

    assert_eq!(result.summary_lines().len(), 1);
}

#[test]
fn an_empty_result_renders_nothing() {
    assert!(ImportResult::default().summary_lines().is_empty());
}

#[test]
fn every_summary_key_resolves() {
    for key in ["import.result.added",
                "import.result.skipped",
                "import.result.unreadable",
                "import.result.unreadable_entry"]
    {
        assert_ne!(t(key), key, "{key} should resolve to its localized copy");
    }
}

#[test]
fn every_reason_key_resolves() {
    for reason in [UnreadableReason::NoFrontmatter,
                   UnreadableReason::NoName,
                   UnreadableReason::EmptyBody,
                   UnreadableReason::Unreadable,
                   UnreadableReason::Malformed]
    {
        let key = reason.l10n_key();
        assert_ne!(t(key), key, "{key} should resolve to its localized copy");
    }
}

/// A missing value would otherwise empty the sentence silently; `t_with`
/// leaves the placeholder visible, and nothing here may rely on that.
#[test]
fn no_placeholder_is_left_unsubstituted() {
    let rendered = full_result().summary_lines().join("\n");

    assert!(!rendered.contains("%{"),
            "every placeholder should have been substituted");
}
