//! Unit tests for [`super`]: the editor's form rules, decided without the
//! editor entity.
//!
//! `name_from_folder` is the whole of "choosing a folder names an unnamed
//! agent after it" - see `openspec/specs/agent-editor-ui/spec.md`. The
//! picker's own method only writes what this returns, so the scenarios are
//! asserted here rather than by opening a window: constructing the editor
//! calls `Settings::load()` against the real platform paths, which a test
//! has no business touching.

use super::name_from_folder;

const FOLDER: &str = "/Users/dana/code/widget";

/// "A blank name takes the folder's name".
#[test]
fn a_blank_name_takes_the_folders_name() {
    assert_eq!(name_from_folder("", FOLDER).as_deref(), Some("widget"));
}

/// "A whitespace-only name counts as blank" - the same test
/// `validated_fields` would have rejected the name by.
#[test]
fn a_whitespace_only_name_counts_as_blank() {
    assert_eq!(name_from_folder("   ", FOLDER).as_deref(), Some("widget"));
    assert_eq!(name_from_folder(" \t\n ", FOLDER).as_deref(),
               Some("widget"));
}

/// "A typed name survives choosing a folder".
#[test]
fn a_typed_name_is_never_replaced() {
    assert_eq!(name_from_folder("Reviewer", FOLDER), None);
}

/// A name the user padded is still a name, not a blank to fill.
#[test]
fn a_padded_name_is_still_a_name() {
    assert_eq!(name_from_folder("  Reviewer  ", FOLDER), None);
}

/// "Correcting the folder does not rename the agent": the second choice sees
/// the name the first one supplied, and so declines.
#[test]
fn a_second_folder_does_not_rename_an_already_filled_field() {
    let first = name_from_folder("", FOLDER).expect("the blank field was filled");
    assert_eq!(first, "widget");

    assert_eq!(name_from_folder(&first, "/Users/dana/code/gadget"),
               None,
               "the name the first folder supplied is a name like any other");
}

/// A folder with no last component leaves the field alone rather than
/// filling it with nothing - the user keeps the blank they had.
#[test]
fn a_folder_with_no_name_leaves_the_field_alone() {
    assert_eq!(name_from_folder("", "/"), None);
    assert_eq!(name_from_folder("", ""), None);
    assert_eq!(name_from_folder("", ".."), None);
}

/// Both reasons to decline at once: nothing is written, and nothing panics.
#[test]
fn a_typed_name_and_a_nameless_folder_change_nothing() {
    assert_eq!(name_from_folder("Reviewer", "/"), None);
}

/// A relative folder names an agent as readily as an absolute one - the
/// dialog's picker returns absolute paths, but the rule does not depend on
/// that.
#[test]
fn a_relative_folder_still_names_the_agent() {
    assert_eq!(name_from_folder("", "code/widget").as_deref(),
               Some("widget"));
}
