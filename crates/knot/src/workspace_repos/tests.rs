//! Unit tests for [`super`]: grouping is pure, and [`super::resolve`] is
//! checked once against a real repository with a linked worktree.

use std::path::PathBuf;
use std::process::Command;

use super::*;

fn slug(owner: &str, repo: &str) -> RepoSlug {
    RepoSlug { owner: owner.to_owned(),
               repo:  repo.to_owned(), }
}

fn folder(toplevel: &str, common_dir: &str, slug: Option<RepoSlug>) -> ResolvedFolder {
    ResolvedFolder { toplevel: PathBuf::from(toplevel),
                     common_dir: PathBuf::from(common_dir),
                     slug }
}

#[test]
fn two_worktrees_of_one_repository_are_one_repository() {
    let repos = group([folder("/src/knot", "/src/knot/.git", Some(slug("acme", "knot"))),
                       folder("/src/knot-worktrees/feature",
                              "/src/knot/.git",
                              Some(slug("acme", "knot")))]);
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].worktrees,
               [PathBuf::from("/src/knot"),
                PathBuf::from("/src/knot-worktrees/feature")]);
    assert_eq!(repos[0].label, "acme/knot");
}

/// Two agents in one tree - one of them in a subfolder, which resolves to
/// the same top level - list the tree once.
#[test]
fn a_tree_shared_by_two_agents_is_listed_once() {
    let repos = group([folder("/src/knot", "/src/knot/.git", None),
                       folder("/src/knot", "/src/knot/.git", None)]);
    assert_eq!(repos[0].worktrees, [PathBuf::from("/src/knot")]);
}

/// Not on GitHub: no slug, and headed by the repository's folder - the same
/// for every worktree, whatever the worktree's own folder is called.
#[test]
fn a_repository_off_github_is_headed_by_its_folder() {
    let repos = group([folder("/work/feature-x", "/src/widget/.git", None)]);
    assert_eq!(repos[0].slug, None);
    assert_eq!(repos[0].label, "widget");
}

#[test]
fn repositories_are_ordered_by_heading() {
    let labels: Vec<_> =
        group([folder("/src/zeta", "/src/zeta/.git", None),
               folder("/src/alpha", "/src/alpha/.git", None)]).into_iter()
                                                              .map(|repo| repo.label)
                                                              .collect();
    assert_eq!(labels, ["alpha", "zeta"]);
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git").args(args)
                                    .current_dir(dir)
                                    .output()
                                    .expect("git runs");
    assert!(status.status.success(), "git {args:?}: {status:?}");
}

/// The whole path against git: a repository, a linked worktree of it, a
/// subfolder, and a folder outside git.
#[test]
fn resolve_reads_worktrees_subfolders_and_skips_non_git_folders() {
    let root = tempfile::tempdir().expect("a temp dir");
    let main = root.path().join("widget");
    std::fs::create_dir_all(main.join("src")).unwrap();
    git(&main, &["init", "-q", "-b", "main"]);
    git(&main,
        &["-c",
          "user.name=t",
          "-c",
          "user.email=t@t",
          "commit",
          "-q",
          "--allow-empty",
          "-m",
          "init"]);
    git(&main,
        &["remote", "add", "origin", "git@github.com:acme/widget.git"]);
    let linked = root.path().join("widget-feature");
    git(&main,
        &["worktree",
          "add",
          "-q",
          linked.to_str().unwrap(),
          "-b",
          "feature"]);
    let outside = root.path().join("notes");
    std::fs::create_dir_all(&outside).unwrap();

    let repos = resolve(&[main.join("src"), linked.clone(), outside]);

    assert_eq!(repos.len(), 1, "{repos:?}");
    assert_eq!(repos[0].slug, Some(slug("acme", "widget")));
    assert_eq!(repos[0].worktrees.len(), 2, "{repos:?}");
}
