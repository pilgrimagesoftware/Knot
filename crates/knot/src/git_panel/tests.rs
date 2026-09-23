use std::path::{Path, PathBuf};

use knot_git::{ChangeType, FileEntry, RepoStatus};

use super::sections::{SectionKind, group, paths_of, selection_survives};
use super::state::{
    DiffKey, DiffOutcome, GitDiffCache, GitStatusCache, GitStatusSnapshot, Selection,
};

fn entry(path: &str, staged: Option<ChangeType>, unstaged: Option<ChangeType>) -> FileEntry {
    FileEntry { path: PathBuf::from(path),
                orig_path: None,
                staged,
                unstaged }
}

fn status(entries: Vec<FileEntry>) -> RepoStatus {
    RepoStatus { head: Some("main".to_owned()),
                 entries,
                 ..RepoStatus::default() }
}

fn kinds(status: &RepoStatus) -> Vec<SectionKind> {
    group(status).into_iter().map(|s| s.kind).collect()
}

/// The case the panel's whole selection model rests on: the sections are
/// independent filters, not a partition, so one path lands in two of them.
#[test]
fn a_staged_and_modified_path_appears_in_both_sections() {
    let status = status(vec![entry("src/f.rs",
                                   Some(ChangeType::Added),
                                   Some(ChangeType::Modified))]);

    let sections = group(&status);

    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].kind, SectionKind::Staged);
    assert_eq!(sections[1].kind, SectionKind::Unstaged);
    assert_eq!(sections[0].rows[0].path, PathBuf::from("src/f.rs"));
    assert_eq!(sections[1].rows[0].path, PathBuf::from("src/f.rs"));
}

/// Each row reports its own side's change type. Swift preferred the staged one
/// for both rows, so the unstaged row lied about what it was.
#[test]
fn each_row_carries_its_own_sides_change_type() {
    let status = status(vec![entry("f.rs", Some(ChangeType::Added), Some(ChangeType::Modified))]);

    let sections = group(&status);

    assert_eq!(sections[0].rows[0].change, ChangeType::Added);
    assert_eq!(sections[1].rows[0].change, ChangeType::Modified);
}

#[test]
fn empty_sections_are_omitted() {
    let only_staged = status(vec![entry("a.rs", Some(ChangeType::Modified), None)]);
    assert_eq!(kinds(&only_staged), [SectionKind::Staged]);

    let only_untracked = status(vec![entry("b.rs", None, Some(ChangeType::Untracked))]);
    assert_eq!(kinds(&only_untracked), [SectionKind::Untracked]);

    assert!(group(&status(Vec::new())).is_empty());
}

#[test]
fn sections_are_in_draw_order() {
    let status = status(vec![entry("d.rs",
                                   Some(ChangeType::Unmerged),
                                   Some(ChangeType::Unmerged)),
                             entry("c.rs", None, Some(ChangeType::Untracked)),
                             entry("b.rs", None, Some(ChangeType::Modified)),
                             entry("a.rs", Some(ChangeType::Added), None)]);

    assert_eq!(kinds(&status),
               [SectionKind::Staged,
                SectionKind::Unstaged,
                SectionKind::Untracked,
                SectionKind::Conflicted]);
}

/// A conflict sets both sides to unmerged, so without taking conflicts out
/// first the same path would also be listed as a staged and an unstaged
/// change - three rows for one conflict.
#[test]
fn a_conflict_is_listed_once() {
    let status = status(vec![entry("f.rs",
                                   Some(ChangeType::Unmerged),
                                   Some(ChangeType::Unmerged))]);

    let sections = group(&status);

    assert_eq!(kinds(&status), [SectionKind::Conflicted]);
    assert_eq!(sections[0].rows.len(), 1);
}

#[test]
fn rows_split_name_from_directory() {
    let status = status(vec![entry("src/deep/f.rs", None, Some(ChangeType::Modified)),
                             entry("top.rs", None, Some(ChangeType::Modified))]);

    let rows = &group(&status)[0].rows;
    let nested = rows.iter().find(|r| r.path.ends_with("f.rs")).unwrap();
    let top = rows.iter().find(|r| r.path == Path::new("top.rs")).unwrap();

    assert_eq!(nested.file_name(), "f.rs");
    assert_eq!(nested.directory().as_deref(), Some("src/deep"));
    assert_eq!(top.file_name(), "top.rs");
    assert_eq!(top.directory(), None, "a root path has no directory line");
}

#[test]
fn selection_survives_a_refresh_that_keeps_its_side() {
    let sections = group(&status(vec![entry("f.rs", Some(ChangeType::Added), None)]));
    let selection = Selection { path:      PathBuf::from("f.rs"),
                                orig_path: None,
                                staged:    true, };

    assert!(selection_survives(&sections, &selection));
}

/// The Swift bug this replaces: it checked only the path, so a file that moved
/// from one side to the other kept a diff that no longer described it.
#[test]
fn selection_is_cleared_when_only_its_other_side_remains() {
    let sections = group(&status(vec![entry("f.rs", Some(ChangeType::Added), None)]));
    let unstaged_side = Selection { path:      PathBuf::from("f.rs"),
                                    orig_path: None,
                                    staged:    false, };

    assert!(!selection_survives(&sections, &unstaged_side),
            "the path is present, but not on the selected side");
}

#[test]
fn selection_is_cleared_when_the_path_is_gone() {
    let sections = group(&status(vec![entry("other.rs", Some(ChangeType::Added), None)]));
    let selection = Selection { path:      PathBuf::from("f.rs"),
                                orig_path: None,
                                staged:    true, };

    assert!(!selection_survives(&sections, &selection));
}

#[test]
fn a_row_selects_its_own_side() {
    let status = status(vec![entry("f.rs", Some(ChangeType::Added), Some(ChangeType::Modified))]);
    let sections = group(&status);

    assert!(sections[0].rows[0].selection().staged);
    assert!(!sections[1].rows[0].selection().staged);
}

#[test]
fn paths_of_collects_every_row() {
    let status = status(vec![entry("a.rs", None, Some(ChangeType::Modified)),
                             entry("b.rs", None, Some(ChangeType::Modified))]);

    let rows = &group(&status)[0].rows;
    let mut paths = paths_of(rows).expect("utf-8 paths");
    paths.sort_unstable();

    assert_eq!(paths, ["a.rs", "b.rs"]);
}

/// Both cache value types have to satisfy `RefreshCache`'s `Clone + PartialEq`
/// bounds. This fails to compile if either stops doing so.
#[test]
fn both_caches_instantiate() {
    let mut statuses = GitStatusCache::default();
    let mut diffs = GitDiffCache::default();
    let agent = uuid::Uuid::new_v4();
    let key = DiffKey { agent,
                        path: PathBuf::from("f.rs"),
                        staged: true };

    let max_age = std::time::Duration::from_secs(1);
    statuses.claim_refresh(agent, max_age)
            .expect("a cold key is claimable")
            .record(GitStatusSnapshot::NotARepository);
    diffs.claim_refresh(key.clone(), max_age)
         .expect("a cold key is claimable")
         .record(DiffOutcome::Absent);

    assert_eq!(statuses.get(&agent),
               Some(GitStatusSnapshot::NotARepository));
    assert_eq!(diffs.get(&key), Some(DiffOutcome::Absent));
}

/// A failure is a value, not an absent entry. `refresh_cache` reserves absence
/// for "no answer yet", so a failure stored as absence leaves the panel on its
/// loading state forever.
#[test]
fn a_failure_is_a_landed_answer() {
    let mut statuses = GitStatusCache::default();
    let agent = uuid::Uuid::new_v4();

    statuses.claim_refresh(agent, std::time::Duration::from_secs(1))
            .unwrap()
            .record(GitStatusSnapshot::Failed("broken".to_owned()));

    assert!(statuses.get(&agent).is_some(),
            "a failure must read as answered, not as still loading");
}
