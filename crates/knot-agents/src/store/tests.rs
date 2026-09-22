mod lifecycle;
mod persistence;
mod pull_requests;
mod workspace;

use super::*;

fn store() -> AgentStore {
    AgentStore::new()
}

#[test]
fn create_from_folder_with_defaults() {
    let mut store = store();
    let id = store.create("/Users/x/proj", CreateOptions::default());
    assert_eq!(store.agent(id).unwrap().name, "proj");
    assert_eq!(store.agent(id).unwrap().agent_type, "claude");
    assert_eq!(store.workspaces()[0].agent_ids, vec![id]);
    assert_eq!(store.workspaces()[0].active_agent_ids, vec![id]);
}

#[test]
fn insert_after_a_sibling() {
    let mut store = store();
    let first = store.create("/tmp/a", CreateOptions::default());
    let last = store.create("/tmp/c", CreateOptions::default());
    let middle = store.create("/tmp/b",
                              CreateOptions { insert_after: Some(first),
                                              ..Default::default() });
    assert_eq!(store.agents()
                    .iter()
                    .map(|agent| agent.id)
                    .collect::<Vec<_>>(),
               vec![first, middle, last]);
    assert_eq!(store.workspaces()[0].agent_ids, vec![first, middle, last]);
}

#[test]
fn setters_apply_and_unknown_ids_are_no_ops() {
    let mut store = store();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_registered(id, true);
    store.set_session_id(id, "session".to_string());
    store.set_status_text(id, "Running tests".to_string());
    store.set_terminal_title(id, "zsh".to_string());
    let agent = store.agent(id).unwrap();
    assert!(agent.is_registered);
    assert_eq!(agent.session_id.as_deref(), Some("session"));
    assert_eq!(agent.status_text, "Running tests");
    assert_eq!(agent.terminal_title, "zsh");
    let stale = Uuid::new_v4();
    store.set_registered(stale, true);
    assert!(store.agent(stale).is_none());
}

#[test]
fn panels_and_bench_behave_as_expected() {
    let mut store = store();
    let id = store.create("/tmp/a", CreateOptions::default());
    let a = std::path::PathBuf::from("/tmp/a.md");
    let b = std::path::PathBuf::from("/tmp/b.md");
    store.set_markdown_panel(id, a.clone(), false).unwrap();
    store.set_markdown_panel(id, b.clone(), false).unwrap();
    store.set_markdown_panel(id, a.clone(), true).unwrap();
    assert_eq!(store.agent(id).unwrap().markdown_history,
               vec![a.clone(), b]);
    assert!(store.agent(id).unwrap().markdown_maximized);
    let bench = knot_core::BenchAgent::new(Uuid::new_v4(), "Ghost", None, "/missing");
    assert!(store.deploy_bench(&bench, |_| false).is_none());
}
