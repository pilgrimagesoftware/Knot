//! Unit tests for [`super`].

use super::KNOT_REPO;

#[test]
fn the_repo_is_a_bare_owner_and_name() {
    let parts: Vec<&str> = KNOT_REPO.split('/').collect();

    assert_eq!(parts.len(), 2, "{KNOT_REPO} is not owner/name");
    assert!(parts.iter().all(|part| !part.is_empty()),
            "{KNOT_REPO} has an empty part");
    assert!(!KNOT_REPO.contains(':'), "{KNOT_REPO} carries a scheme");
}

#[test]
fn the_repo_matches_the_manifest() {
    let manifest = env!("CARGO_PKG_REPOSITORY").trim_end_matches('/');

    assert!(manifest.ends_with(&format!("/{KNOT_REPO}")),
            "Cargo.toml names {manifest}, KNOT_REPO names {KNOT_REPO}");
}
