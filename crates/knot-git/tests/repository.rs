use std::path::Path;

use knot_git::Repository;

fn init_repo(dir: &Path) {
    let run = |args: &[&str]| {
        knot_git::Runner::new(dir).run(args).unwrap();
    };
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
    std::fs::write(dir.join("seed.txt"), "seed\n").unwrap();
    run(&["add", "-A"]);
    run(&["commit", "-qm", "init"]);
}

#[test]
fn toplevel_resolves_a_subfolder_to_the_repository_root() {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path().join("repo");
    std::fs::create_dir(&repo_path).unwrap();
    init_repo(&repo_path);
    let sub = repo_path.join("sub");
    std::fs::create_dir(&sub).unwrap();

    let top = Repository::open(&sub).toplevel().unwrap().unwrap();

    assert_eq!(std::fs::canonicalize(top).unwrap(),
               std::fs::canonicalize(&repo_path).unwrap());
}

#[test]
fn toplevel_is_none_outside_a_repository() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(Repository::open(dir.path()).toplevel().unwrap(), None);
}

#[test]
fn common_dir_is_none_outside_a_repository() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(Repository::open(dir.path()).common_dir().unwrap(), None);
}

#[test]
fn linked_worktrees_share_one_common_dir() {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path().join("repo");
    std::fs::create_dir(&repo_path).unwrap();
    init_repo(&repo_path);
    let repo = Repository::open(&repo_path);
    let worktree_path = dir.path().join("repo-feature");
    repo.create_worktree("feature", &worktree_path).unwrap();

    let primary_common = repo.common_dir().unwrap().unwrap();
    let worktree_common = Repository::open(&worktree_path).common_dir()
                                                          .unwrap()
                                                          .unwrap();

    assert_eq!(primary_common, worktree_common);
    assert!(primary_common.is_absolute());
}

#[test]
fn remote_url_reads_origin() {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path().join("repo");
    std::fs::create_dir(&repo_path).unwrap();
    init_repo(&repo_path);
    let repo = Repository::open(&repo_path);
    knot_git::Runner::new(&repo_path).run(&["remote",
                                            "add",
                                            "origin",
                                            "git@github.com:acme/widget.git"])
                                     .unwrap();

    assert_eq!(repo.remote_url("origin").unwrap().as_deref(),
               Some("git@github.com:acme/widget.git"));
}

#[test]
fn remote_url_is_none_without_the_remote() {
    let dir = tempfile::tempdir().unwrap();
    let repo_path = dir.path().join("repo");
    std::fs::create_dir(&repo_path).unwrap();
    init_repo(&repo_path);

    assert_eq!(Repository::open(&repo_path).remote_url("origin").unwrap(),
               None);
}

#[test]
fn remote_url_is_none_outside_a_repository() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(Repository::open(dir.path()).remote_url("origin").unwrap(),
               None);
}
