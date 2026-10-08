//! Unit tests for [`super`]: one test per scenario under "Un-archived
//! changes are found in the workspace's folders".

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{ChangeEntry, merge, scan};

/// `<worktree>/openspec/changes/<name>[/proposal.md]`.
fn write_change(worktree: &Path, name: &str, proposal: Option<&str>) {
    let dir = worktree.join("openspec").join("changes").join(name);
    fs::create_dir_all(&dir).expect("create change dir");
    if let Some(proposal) = proposal {
        fs::write(dir.join("proposal.md"), proposal).expect("write proposal.md");
    }
}

#[test]
fn lists_a_change() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);

    let changes = scan(worktree.path());

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].name, "add-login");
    assert_eq!(changes[0].dir,
               worktree.path()
                       .join("openspec")
                       .join("changes")
                       .join("add-login"));
}

#[test]
fn excludes_the_archive_directory() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);
    write_change(worktree.path(), "archive", None);
    write_change(&worktree.path()
                          .join("openspec")
                          .join("changes")
                          .join("archive"),
                 "2026-09-01-old-work",
                 None);

    let changes = scan(worktree.path());

    assert_eq!(changes.iter()
                      .map(|change| change.name.as_str())
                      .collect::<Vec<_>>(),
               vec!["add-login"]);
}

#[test]
fn excludes_dot_directories() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);
    write_change(worktree.path(), ".DS_Store_dir", None);

    let changes = scan(worktree.path());

    assert_eq!(changes.iter()
                      .map(|change| change.name.as_str())
                      .collect::<Vec<_>>(),
               vec!["add-login"]);
}

#[test]
fn ignores_plain_files() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);
    fs::write(worktree.path()
                      .join("openspec")
                      .join("changes")
                      .join("README.md"),
              "notes").expect("write file");

    let changes = scan(worktree.path());

    assert_eq!(changes.iter()
                      .map(|change| change.name.as_str())
                      .collect::<Vec<_>>(),
               vec!["add-login"]);
}

#[test]
fn a_missing_changes_directory_yields_an_empty_list() {
    let worktree = TempDir::new().expect("tempdir");

    assert_eq!(scan(worktree.path()), Vec::new());
}

#[test]
fn why_is_the_first_non_empty_line_after_the_heading() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(),
                 "add-login",
                 Some("# Add Login\n\n## Why\n\nUsers cannot sign in with SSO.\n\nMore detail.\n\n## What Changes\n\nSomething else.\n"));

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why,
               Some("Users cannot sign in with SSO.".to_string()));
}

#[test]
fn why_is_none_with_no_proposal() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(), "add-login", None);

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why, None);
}

#[test]
fn why_is_none_with_no_heading() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(),
                 "add-login",
                 Some("# Add Login\n\nNo Why section here.\n"));

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why, None);
}

#[test]
fn why_is_none_with_an_empty_heading() {
    let worktree = TempDir::new().expect("tempdir");
    write_change(worktree.path(),
                 "add-login",
                 Some("# Add Login\n\n## Why\n\n## What Changes\n\nSomething else.\n"));

    let changes = scan(worktree.path());

    assert_eq!(changes[0].why, None);
}

#[test]
fn a_change_in_two_worktrees_merges_to_one_entry_keeping_the_first() {
    let first = TempDir::new().expect("tempdir");
    let second = TempDir::new().expect("tempdir");
    write_change(first.path(), "add-login", None);
    write_change(second.path(), "add-login", None);

    let merged = merge([scan(first.path()), scan(second.path())]);

    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].dir,
               first.path()
                    .join("openspec")
                    .join("changes")
                    .join("add-login"));
}

#[test]
fn merge_keeps_changes_unique_to_either_worktree() {
    let first = TempDir::new().expect("tempdir");
    let second = TempDir::new().expect("tempdir");
    write_change(first.path(), "add-login", None);
    write_change(second.path(), "add-export", None);

    let merged = merge([scan(first.path()), scan(second.path())]);

    assert_eq!(merged.iter()
                     .map(|change: &ChangeEntry| change.name.as_str())
                     .collect::<Vec<_>>(),
               vec!["add-export", "add-login"]);
}
