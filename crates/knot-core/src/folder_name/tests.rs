//! Unit tests for [`super::folder_name`].
//!
//! Moved here with the function from `knot-agents`'s store helpers, where it
//! was `last_path_component`. The `None` cases are new: the store never
//! needed to tell them apart from a name, and the editor does.

use super::folder_name;

#[test]
fn a_folder_is_named_by_its_last_component() {
    assert_eq!(folder_name("/Users/dana/code/widget").as_deref(),
               Some("widget"));
    assert_eq!(folder_name("widget").as_deref(), Some("widget"));
}

#[test]
fn a_trailing_separator_does_not_hide_the_name() {
    assert_eq!(folder_name("/Users/dana/code/widget/").as_deref(),
               Some("widget"));
}

/// The case the editor depends on: nothing to name the agent after, so it
/// must be told that rather than handed a plausible-looking string.
#[test]
fn a_path_with_no_last_component_yields_none() {
    assert_eq!(folder_name("/"), None);
    assert_eq!(folder_name(""), None);
    assert_eq!(folder_name(".."), None);
}

/// A name that is not valid UTF-8 still names the folder - lossily, but the
/// alternative is an agent the user cannot name by choosing its folder.
#[test]
fn a_name_survives_lossy_conversion() {
    assert_eq!(folder_name("/tmp/naïve").as_deref(), Some("naïve"));
}
