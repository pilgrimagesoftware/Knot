//! Unit tests for [`super`].

use std::path::PathBuf;

use super::*;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter()
         .map(|name| PathBuf::from(format!("/Users/me/Desktop/{name}")))
         .collect()
}

#[test]
fn images_are_kept_and_the_rest_are_skipped() {
    let mut chosen = Vec::new();

    let skipped = add(&mut chosen,
                      paths(&["one.png", "notes.txt", "two.JPG", "three.gif"]));

    assert_eq!(chosen, paths(&["one.png", "two.JPG", "three.gif"]));
    assert_eq!(skipped, 1, "the text file is the one skipped");
}

#[test]
fn a_file_without_an_extension_is_not_an_image() {
    let mut chosen = Vec::new();
    assert_eq!(add(&mut chosen, paths(&["Screenshot"])), 1);
    assert!(chosen.is_empty());
}

#[test]
fn the_cap_skips_what_does_not_fit() {
    let mut chosen = Vec::new();
    let picked = (0..BUG_REPORT_MAX_SCREENSHOTS + 2).map(|index| {
                                                         PathBuf::from(format!("/shots/{index}.png"))
                                                     });

    let skipped = add(&mut chosen, picked);

    assert_eq!(chosen.len(), BUG_REPORT_MAX_SCREENSHOTS);
    assert_eq!(skipped, 2);
}

/// Picking a file again is not an error, and does not list it twice.
#[test]
fn picking_one_again_changes_nothing() {
    let mut chosen = paths(&["one.png"]);
    assert_eq!(add(&mut chosen, paths(&["one.png"])), 0);
    assert_eq!(chosen, paths(&["one.png"]));
}

#[test]
fn the_body_names_files_not_paths() {
    let section = body_section(&paths(&["one.png", "two.png"]));

    assert!(section.contains(&knot_core::l10n::t("bug_report.screenshots.label")));
    assert!(section.contains(&knot_core::l10n::t("bug_report.screenshots.body_note")));
    assert!(section.contains("`one.png`") && section.contains("`two.png`"),
            "{section}");
    assert!(!section.contains("/Users/me"),
            "a path leaked into the body: {section}");
}

#[test]
fn no_screenshots_add_no_section() {
    assert_eq!(body_section(&[]), "");
}

#[test]
fn the_handoff_follows_the_count_and_the_platform() {
    assert_eq!(Handoff::for_report(0, true), Handoff::None);
    assert_eq!(Handoff::for_report(0, false), Handoff::None);
    assert_eq!(Handoff::for_report(2, true), Handoff::Shown);
    assert_eq!(Handoff::for_report(2, false), Handoff::ByHand);
}
