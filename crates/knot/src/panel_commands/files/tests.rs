//! Enumerating a folder: which files are offered, and which are not.
//!
//! These run `git` against a temporary repository, so they need `git` on
//! `PATH` the same way `knot-git`'s own tests do.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::panel_commands::LookupSource;
use crate::panel_commands::files::FolderFiles;

/// Runs `git <args>` in `dir`, failing the test if git does.
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git").args(args)
                                    .current_dir(dir)
                                    .status()
                                    .expect("git should be on PATH");
    assert!(status.success(), "git {args:?} failed");
}

/// Writes `dir/path`, creating parents.
fn write(dir: &Path, path: &str, contents: &str) {
    let full = dir.join(path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(full, contents).expect("write file");
}

/// A repository with one committed file, a `.gitignore` that ignores
/// `target/`, and an ignored file inside it.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path();
    git(path, &["init", "--quiet"]);
    git(path, &["config", "user.email", "test@example.com"]);
    git(path, &["config", "user.name", "Test"]);

    write(path, "src/lib.rs", "// tracked\n");
    write(path, ".gitignore", "target/\n");
    write(path, "target/debug/build.log", "ignored\n");

    git(path, &["add", "src/lib.rs", ".gitignore"]);
    git(path, &["commit", "--quiet", "-m", "initial"]);
    dir
}

/// The paths `FolderFiles` offers for `folder`.
fn tokens(folder: &Path) -> Vec<String> {
    FolderFiles::enumerate(folder).entries()
                                  .into_iter()
                                  .map(|entry| entry.token)
                                  .collect()
}

/// Task 6.4's own check, and the reason enumeration goes through
/// `knot-git` rather than a walk: an ignored path never appears.
#[test]
fn an_ignored_path_is_not_offered() {
    let dir = repository();

    let found = tokens(dir.path());

    assert!(found.iter().any(|path| path == "src/lib.rs"),
            "the tracked file should be offered, or this proves nothing");
    assert!(!found.iter().any(|path| path.starts_with("target/")),
            "`target/` is ignored by the repository, so it must not reach the popup: {found:?}");
}

/// The other half: a file created a moment ago and never committed is
/// exactly what a prompt is most likely to be about.
#[test]
fn a_new_untracked_file_is_offered() {
    let dir = repository();
    write(dir.path(), "notes/today.md", "just written\n");

    let found = tokens(dir.path());

    assert!(found.iter().any(|path| path == "notes/today.md"),
            "an untracked but unignored file should be offered: {found:?}");
}

/// `.git` itself is never interesting, and git does not list it.
#[test]
fn the_git_directory_is_never_offered() {
    let dir = repository();

    let found = tokens(dir.path());

    assert!(!found.iter().any(|path| path.starts_with(".git/")),
            "{found:?}");
}

/// A folder that is not a repository falls back to a walk, which has to
/// find ordinary files and skip the excluded directory names.
#[test]
fn a_plain_folder_is_walked_with_the_exclusion_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "README.md", "hello\n");
    write(dir.path(), "src/main.rs", "fn main() {}\n");
    write(dir.path(),
          "node_modules/left-pad/index.js",
          "module.exports = 1\n");
    write(dir.path(), "DerivedData/build/out.o", "binary\n");

    let found = tokens(dir.path());

    assert!(found.iter().any(|path| path == "README.md"), "{found:?}");
    assert!(found.iter().any(|path| path == "src/main.rs"), "{found:?}");
    assert!(!found.iter().any(|path| path.starts_with("node_modules")),
            "the Swift reference skips `node_modules`, so this does too: {found:?}");
    assert!(!found.iter().any(|path| path.starts_with("DerivedData")),
            "{found:?}");
}

/// A folder that does not exist is empty rather than an error: an agent
/// may be pointed at a folder that has been moved, and the composer has to
/// keep working.
#[test]
fn a_missing_folder_yields_nothing_without_failing() {
    let files = FolderFiles::enumerate(Path::new("/no/such/folder/anywhere"));

    assert!(files.entries().is_empty());
    assert!(!files.overflowed(),
            "nothing found is not the same as too much found");
}

/// Enumeration does not rank - that is the matcher's job - but it must not
/// lose paths either.
#[test]
fn every_file_in_a_plain_folder_is_offered_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "a.rs", "");
    write(dir.path(), "deep/b.rs", "");
    write(dir.path(), "deep/deeper/c.rs", "");

    let mut found = tokens(dir.path());
    found.sort();

    assert_eq!(found, vec!["a.rs", "deep/b.rs", "deep/deeper/c.rs"]);
}
