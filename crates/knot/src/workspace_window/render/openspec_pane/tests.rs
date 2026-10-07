//! A change row's Copy name and Reveal in Finder (`workspace-openspec-changes`,
//! "A change row offers its actions"), against changes scanned from disk.

use std::path::Path;

use super::change_actions;
use crate::openspec_changes;

fn propose(tree: &Path, name: &str) {
    std::fs::create_dir_all(tree.join("openspec/changes").join(name)).unwrap();
}

/// Copy name yields the name, and Reveal targets the change's directory in
/// the first working tree that holds it, when two do.
#[test]
fn copy_name_is_the_name_and_reveal_is_the_first_worktree() {
    let root = tempfile::tempdir().expect("a temp dir");
    let (main, feature) = (root.path().join("main"), root.path().join("feature"));
    propose(&main, "add-login");
    propose(&feature, "add-login");

    let merged = openspec_changes::merge([openspec_changes::scan(&main),
                                          openspec_changes::scan(&feature)]);
    assert_eq!(merged.len(), 1, "one change across two worktrees");

    let (copied, revealed) = change_actions(&merged[0]);
    assert_eq!(copied, "add-login");
    assert_eq!(revealed, main.join("openspec/changes/add-login"));
}
