//! Unit tests for [`super`].

use super::*;

#[test]
fn tags_are_trimmed_lowercased_and_deduplicated() {
    let tags: Capabilities = [" Rust ", "rust", ""].iter().collect();

    assert_eq!(tags.len(), 1);
    assert!(tags.contains("rust"));
}

#[test]
fn a_tag_that_normalizes_to_nothing_is_not_stored() {
    let mut tags = Capabilities::new();

    assert!(!tags.insert("   "));
    assert!(!tags.insert(""));
    assert!(tags.is_empty());
}

/// An agent tagged one way must be found by a query written the other.
#[test]
fn lookup_matches_however_the_caller_spelled_it() {
    let tags: Capabilities = ["Code-Review"].iter().collect();

    assert!(tags.contains("code-review"));
    assert!(tags.contains("  CODE-REVIEW  "));
    assert!(!tags.contains("code review"));
}

#[test]
fn contains_all_requires_every_requested_tag() {
    let agent: Capabilities = ["rust", "testing"].iter().collect();

    assert!(agent.contains_all(&["rust"].iter().collect()));
    assert!(agent.contains_all(&["rust", "testing"].iter().collect()));
    assert!(!agent.contains_all(&["rust", "infrastructure"].iter().collect()));
}

/// An empty request matches everything, which is how "list them all" works.
#[test]
fn contains_all_is_trivially_true_for_no_requested_tags() {
    let agent: Capabilities = ["rust"].iter().collect();

    assert!(agent.contains_all(&Capabilities::new()));
    assert!(Capabilities::new().contains_all(&Capabilities::new()));
}

#[test]
fn removal_matches_on_the_normalized_form() {
    let mut tags: Capabilities = ["rust"].iter().collect();

    assert!(tags.remove(" RUST "));
    assert!(tags.is_empty());
    assert!(!tags.remove("rust"));
}

#[test]
fn serializes_as_a_sorted_array() {
    let tags: Capabilities = ["testing", "rust", "code-review"].iter().collect();

    assert_eq!(serde_json::to_string(&tags).unwrap(),
               r#"["code-review","rust","testing"]"#);
}

/// A hand-edited settings file must not be able to introduce a tag no query
/// will ever match.
#[test]
fn deserializing_normalizes_what_it_finds() {
    let tags: Capabilities = serde_json::from_str(r#"[" Rust ","RUST","","testing"]"#).unwrap();

    assert_eq!(tags.len(), 2);
    assert!(tags.contains("rust"));
    assert!(tags.contains("testing"));
}
