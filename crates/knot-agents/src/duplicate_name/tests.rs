//! Unit tests for [`super`]. Names are built from the catalog entry, so these
//! assert the shape the entry produces rather than hardcoding English copy -
//! `render` is the same function the code under test uses.

use super::{duplicate_name, render, stem};

/// The name the catalog gives a stem and a number, for tests to expect
/// without spelling the separator themselves.
fn numbered(stem: &str, number: usize) -> String {
    render(stem, &number.to_string())
}

/// The bug: `(copy)` stacked, so three duplications read
/// `Foo (copy) (copy) (copy)`.
#[test]
fn duplicating_a_duplicate_extends_the_series_rather_than_nesting() {
    let first = duplicate_name("Foo", []);
    assert_eq!(first, numbered("Foo", 2));

    let second = duplicate_name(&first, [first.as_str()]);
    assert_eq!(second, numbered("Foo", 3));

    let third = duplicate_name(&second, [first.as_str(), second.as_str()]);
    assert_eq!(third, numbered("Foo", 4));
}

#[test]
fn the_first_duplicate_of_an_unnumbered_name_is_two() {
    assert_eq!(duplicate_name("Foo", ["Foo"]), numbered("Foo", 2));
}

#[test]
fn a_taken_number_is_skipped() {
    let two = numbered("Foo", 2);
    let three = numbered("Foo", 3);

    assert_eq!(duplicate_name("Foo", ["Foo", two.as_str(), three.as_str()]),
               numbered("Foo", 4));
}

/// The lowest free number, not one past the source's: a user who deleted the
/// middle of a series is looking at the gap.
#[test]
fn a_gap_in_the_series_is_reused() {
    let five = numbered("Foo", 5);

    assert_eq!(duplicate_name(&five, ["Foo", five.as_str()]),
               numbered("Foo", 2));
}

/// Duplicating from the middle of a series extends the series.
#[test]
fn duplicating_a_numbered_name_numbers_from_its_stem() {
    let two = numbered("Foo", 2);

    assert_eq!(duplicate_name(&two, ["Foo", two.as_str()]),
               numbered("Foo", 3));
}

#[test]
fn a_name_with_spaces_keeps_all_of_them() {
    assert_eq!(duplicate_name("My Best Agent", []),
               numbered("My Best Agent", 2));
}

/// A trailing word is not a series number.
#[test]
fn a_non_numeric_tail_is_part_of_the_stem() {
    assert_eq!(stem("My Agent"), "My Agent");
    assert_eq!(duplicate_name("My Agent", []), numbered("My Agent", 2));
}

#[test]
fn a_trailing_number_is_stripped_from_the_stem() {
    assert_eq!(stem(&numbered("Foo", 2)), "Foo");
    assert_eq!(stem(&numbered("My Best Agent", 11)), "My Best Agent");
}

/// A name that is only a number keeps it: stripping would leave an empty
/// stem, and an agent called "2" duplicating to " 2" helps nobody.
#[test]
fn a_name_that_is_only_a_number_is_left_alone() {
    assert_eq!(stem("7"), "7");
    assert_eq!(duplicate_name("7", []), numbered("7", 2));
}

#[test]
fn an_empty_name_still_produces_something() {
    assert_eq!(duplicate_name("", []), numbered("", 2));
}

/// The catalog owns the shape: the key has to resolve, and both placeholders
/// have to be substituted. Asserting the key resolves, never the English.
#[test]
fn the_name_comes_from_the_catalog_with_both_values_substituted() {
    let rendered = numbered("Foo", 2);

    assert!(!rendered.contains("agent.duplicate_name"),
            "catalog key did not resolve: {rendered}");
    assert!(!rendered.contains("%{name}") && !rendered.contains("%{number}"),
            "placeholder left unsubstituted: {rendered}");
    assert!(rendered.contains("Foo") && rendered.contains('2'),
            "value missing from the rendered name: {rendered}");
}
