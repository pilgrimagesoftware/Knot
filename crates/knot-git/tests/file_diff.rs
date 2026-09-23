//! `Repository::file_diff` against a real repository: which side of the status
//! a diff comes from, and the pathspec separator that keeps a path from being
//! read as a revision.

use std::fs;
use std::path::Path;

use knot_git::{LineKind, Repository, Runner};

fn init_repo(dir: &Path) {
    let run = |args: &[&str]| {
        Runner::new(dir).run(args).unwrap();
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
}

/// A repository with one committed file, ready to be dirtied.
fn repo_with_committed_file(dir: &Path, name: &str, contents: &str) -> Repository {
    init_repo(dir);
    fs::write(dir.join(name), contents).unwrap();
    let run = |args: &[&str]| {
        Runner::new(dir).run(args).unwrap();
    };
    run(&["add", "-A"]);
    run(&["commit", "-qm", "init"]);
    Repository::open(dir)
}

fn added_lines(diff: &knot_git::FileDiff) -> Vec<String> {
    diff.hunks
        .iter()
        .flat_map(|h| &h.lines)
        .filter(|l| l.kind == LineKind::Addition)
        .map(|l| l.text.clone())
        .collect()
}

#[test]
fn unstaged_change_yields_a_hunk() {
    let dir = tempfile::tempdir().unwrap();
    let repo = repo_with_committed_file(dir.path(), "f.txt", "one\n");
    fs::write(dir.path().join("f.txt"), "one\ntwo\n").unwrap();

    let diff = repo.file_diff("f.txt", None, false)
                   .unwrap()
                   .expect("a modified file has a worktree diff");

    assert_eq!(diff.path, Path::new("f.txt"));
    assert!(!diff.binary);
    assert_eq!(diff.additions(), 1);
    assert_eq!(diff.deletions(), 0);
    assert_eq!(added_lines(&diff), ["two"]);
}

#[test]
fn unchanged_path_has_no_diff() {
    let dir = tempfile::tempdir().unwrap();
    let repo = repo_with_committed_file(dir.path(), "f.txt", "one\n");

    assert!(repo.file_diff("f.txt", None, false).unwrap().is_none());
    assert!(repo.file_diff("f.txt", None, true).unwrap().is_none());
}

/// The case the panel's two rows for one path rest on: staged and unstaged are
/// different diffs, and each side must report only its own change.
#[test]
fn staged_and_unstaged_sides_differ() {
    let dir = tempfile::tempdir().unwrap();
    let repo = repo_with_committed_file(dir.path(), "f.txt", "one\n");

    fs::write(dir.path().join("f.txt"), "one\nstaged\n").unwrap();
    repo.stage(&["f.txt"]).unwrap();
    fs::write(dir.path().join("f.txt"), "one\nstaged\nunstaged\n").unwrap();

    let staged = repo.file_diff("f.txt", None, true).unwrap().unwrap();
    let unstaged = repo.file_diff("f.txt", None, false).unwrap().unwrap();

    assert_eq!(added_lines(&staged), ["staged"]);
    assert_eq!(added_lines(&unstaged), ["unstaged"]);
}

#[test]
fn untracked_file_has_no_diff_until_staged() {
    let dir = tempfile::tempdir().unwrap();
    let repo = repo_with_committed_file(dir.path(), "f.txt", "one\n");
    fs::write(dir.path().join("new.txt"), "fresh\n").unwrap();

    // git does not diff an untracked path on either side.
    assert!(repo.file_diff("new.txt", None, false).unwrap().is_none());
    assert!(repo.file_diff("new.txt", None, true).unwrap().is_none());

    repo.stage(&["new.txt"]).unwrap();
    let staged = repo.file_diff("new.txt", None, true).unwrap().unwrap();
    assert_eq!(added_lines(&staged), ["fresh"]);
}

/// Without the `--` separator git reads `release` as the branch and diffs the
/// working tree against it, so the wrong content comes back rather than an
/// error. Agent branches and file names collide often enough for this to be a
/// real failure, not a theoretical one.
#[test]
fn a_path_that_also_names_a_branch_is_read_as_a_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    let repo = repo_with_committed_file(path, "release", "v1\n");
    Runner::new(path).run(&["branch", "release"]).unwrap();

    fs::write(path.join("release"), "v1\nv2\n").unwrap();

    let diff = repo.file_diff("release", None, false)
                   .unwrap()
                   .expect("the file, not the branch");

    assert_eq!(diff.path, Path::new("release"));
    assert_eq!(added_lines(&diff), ["v2"]);
}

#[test]
fn binary_file_is_flagged_rather_than_diffed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    let repo = repo_with_committed_file(path, "f.txt", "one\n");

    fs::write(path.join("blob.bin"), [0u8, 159, 146, 150, 0, 1, 2]).unwrap();
    repo.stage(&["blob.bin"]).unwrap();

    let diff = repo.file_diff("blob.bin", None, true).unwrap().unwrap();

    assert!(diff.binary, "a binary file must be flagged, not diffed");
    assert!(diff.hunks.is_empty());
}

/// Git detects a rename by comparing both sides, so the destination alone is
/// not enough: scoped to it, git reports a whole new file. Passing the source
/// from `FileEntry::orig_path` is what keeps the rename visible.
#[test]
fn a_rename_needs_its_source_to_stay_a_rename() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();
    let repo = repo_with_committed_file(path, "old.txt", "keep\n");

    fs::rename(path.join("old.txt"), path.join("new.txt")).unwrap();
    repo.stage_all().unwrap();

    let destination_only = repo.file_diff("new.txt", None, true).unwrap().unwrap();
    assert_eq!(destination_only.old_path, None,
               "scoped to the destination, git cannot see the rename");
    assert_eq!(destination_only.additions(),
               1,
               "it reports a new file instead");

    let both_sides = repo.file_diff("new.txt", Some("old.txt"), true)
                         .unwrap()
                         .unwrap();

    assert_eq!(both_sides.path, Path::new("new.txt"));
    assert_eq!(both_sides.old_path.as_deref(), Some(Path::new("old.txt")));
    assert_eq!(both_sides.additions(), 0, "a pure rename changes no lines");
}
