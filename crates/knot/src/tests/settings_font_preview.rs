//! The Appearance tab's font pickers preview the family they name.
//!
//! What a row shows is decided by `font_label` alone - the family to draw the
//! label in, and the text of the label itself - so the rule can be asserted
//! here without a window, a text system, or an installed font.

use crate::settings_window::font::font_label;

/// Standing in for `cx.text_system().all_font_names()`.
fn resolvable() -> Vec<String> {
    vec!["Helvetica Neue".to_string(), "JetBrains Mono".to_string()]
}

#[test]
fn a_resolvable_family_is_previewed_and_not_marked() {
    let label = font_label("Helvetica Neue", 13.0, &resolvable());

    assert_eq!(label.family.as_deref(), Some("Helvetica Neue"));
    assert_eq!(label.text, "Helvetica Neue, 13pt");
}

#[test]
fn an_unresolvable_family_is_named_marked_and_left_in_the_default_face() {
    let label = font_label("Not Installed", 13.0, &resolvable());

    assert_eq!(label.family, None);
    // The whole string, so the marking's own text is pinned too: a stale
    // locale catalog would leave `t` returning the bare key.
    assert_eq!(label.text, "Not Installed, 13pt (unavailable)");
}

/// An empty `ui_font_name` is not a family the text system can resolve, so it
/// takes the same route as an uninstalled one rather than a third.
#[test]
fn an_empty_family_is_treated_as_unresolvable() {
    let label = font_label("", 13.0, &resolvable());

    assert_eq!(label.family, None);
    assert_ne!(label.text, ", 13pt", "an empty family must be marked too");
}

/// The size is stated in the label and never applied to it - a 48pt title
/// font must not make its row taller than the others.
#[test]
fn the_size_is_stated_in_the_label_but_carries_no_face_of_its_own() {
    let label = font_label("Helvetica Neue", 48.0, &resolvable());

    assert_eq!(label.text, "Helvetica Neue, 48pt");
    assert_eq!(label.family.as_deref(), Some("Helvetica Neue"));
}
