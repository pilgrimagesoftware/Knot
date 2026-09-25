use std::path::Path;

use knot_git::Runner;
use uuid::Uuid;

use super::*;

fn library() -> Vec<Prompt> {
    vec![Prompt::new("gate", "run {{branch}}").unwrap()]
}

#[test]
fn resolution_covers_each_form() {
    let library = library();
    let live = StartupPrompt::Library(library[0].id);
    let dangling = StartupPrompt::Library(Uuid::new_v4());
    let custom = StartupPrompt::Custom("mine".into());

    assert_eq!(resolve_startup_prompt(None, &library), None);
    assert_eq!(resolve_startup_prompt(Some(&live), &library).as_deref(),
               Some("run {{branch}}"));
    assert_eq!(resolve_startup_prompt(Some(&dangling), &library), None);
    assert_eq!(resolve_startup_prompt(Some(&custom), &library).as_deref(),
               Some("mine"));
}

fn init_repo(dir: &Path) {
    let run = |args: &[&str]| {
        Runner::new(dir).run(args).unwrap();
    };
    run(&["init", "-q", "-b", "471-restart"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "commit.gpgsign", "false"]);
    run(&["commit", "-qm", "root", "--allow-empty"]);
}

fn source(folder: &Path) -> ContextSource {
    ContextSource { agent_name: "Knot 3".into(),
                    folder: folder.to_string_lossy().into_owned(),
                    ..Default::default() }
}

#[test]
fn the_context_reads_the_branch() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());

    let ctx = read_context(source(dir.path()));

    assert_eq!(ctx.branch, "471-restart");
    assert_eq!(ctx.agent_name, "Knot 3");
    assert_eq!(ctx.workspace, "");
}

#[test]
fn a_detached_head_is_named_by_its_short_commit() {
    let dir = tempfile::tempdir().unwrap();
    init_repo(dir.path());
    let full = Runner::new(dir.path()).run(&["rev-parse", "HEAD"]).unwrap();
    Runner::new(dir.path()).run(&["checkout", "--detach", "-q"])
                           .unwrap();

    let branch = read_context(source(dir.path())).branch;

    assert!(!branch.is_empty() && full.starts_with(&branch), "{branch}");
}

#[test]
fn a_folder_outside_a_repository_has_no_branch() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(read_context(source(dir.path())).branch, "");
}

#[test]
fn the_date_is_iso_formatted() {
    let date = read_context(source(Path::new("/nonexistent"))).date;
    let parts: Vec<&str> = date.split('-').collect();
    assert_eq!(parts.len(), 3, "{date}");
    assert_eq!((parts[0].len(), parts[1].len(), parts[2].len()), (4, 2, 2));
}
