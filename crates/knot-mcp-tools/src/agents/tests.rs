use std::path::Path;

use knot_agents::{AgentStore, CreateOptions};
use knot_core::BenchAgent;
use knot_git::Runner;
use serde_json::json;
use uuid::Uuid;

use super::*;

#[test]
fn register_agent_returns_roster() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    let result = register_agent(&mut store, &json!({"agentId": id.to_string()}));
    assert_eq!(result.is_error, None);
    assert!(store.agent(id).unwrap().is_registered);
    assert!(result.content[0].text.contains("knotMembers"));
}

#[test]
fn list_agents_excludes_unowned_companions() {
    let mut store = AgentStore::new();
    let owner = store.create("/tmp/owner", CreateOptions::default());
    let other = store.create("/tmp/other",
                             CreateOptions { insert_after: Some(owner),
                                             ..Default::default() });
    store.create("/tmp/comp",
                 CreateOptions { is_companion: true,
                                 created_by: Some(owner),
                                 insert_after: Some(owner),
                                 ..Default::default() });
    let result = list_agents(&store, &json!({"agentId": other.to_string()}));
    assert_eq!(result.is_error, None);
    assert!(!result.content[0].text.contains("comp"));
}

#[test]
fn list_agents_unknown_caller_errors() {
    let result = list_agents(&AgentStore::new(), &json!({"agentId": "nope"}));
    assert_eq!(result.is_error, Some(true));
}

#[test]
fn create_agent_from_explicit_fields() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let result = create_agent(&mut store,
                              &json!({"agentId": caller.to_string(), "name": "worker", "agentType": "claude", "repoPath": "/tmp/worker"}),
                              &[]);
    assert_eq!(result.is_error, None);
    assert_eq!(store.agents()
                    .iter()
                    .find(|agent| agent.name == "worker")
                    .unwrap()
                    .created_by,
               Some(caller));
}

#[test]
fn create_agent_from_bench_template() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let bench_id = Uuid::new_v4();
    let bench = BenchAgent { id:            bench_id,
                             name:          "Bench Worker".to_string(),
                             avatar:        "🤖".to_string(),
                             folder:        "/tmp/bench".to_string(),
                             agent_type:    "codex".to_string(),
                             shell_command: None,
                             persona_id:    None, };
    let result = create_agent(&mut store,
                              &json!({"agentId": caller.to_string(), "benchAgentId": bench_id.to_string()}),
                              &[bench]);
    assert_eq!(result.is_error, None);
    let created = store.agents()
                       .iter()
                       .find(|agent| agent.name == "Bench Worker")
                       .unwrap();
    assert_eq!(created.folder, "/tmp/bench");
    assert_eq!(created.agent_type, "codex");
}

#[test]
fn create_agent_worktree_without_branch_name_errors() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let result = create_agent(&mut store,
                              &json!({"agentId": caller.to_string(), "name": "worker", "agentType": "claude", "repoPath": "/tmp/worker", "createWorktree": true}),
                              &[]);
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("branchName"));
}

#[test]
fn companion_requires_shell_agent_type() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let result = create_agent(&mut store,
                              &json!({"agentId": caller.to_string(), "name": "companion", "agentType": "claude", "repoPath": "/tmp/companion", "companion": true}),
                              &[]);
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("agentType=shell"));
}

#[test]
fn create_agent_uses_new_worktree_path() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let run = |args: &[&str]| Runner::new(&repo).run(args).unwrap();
    run(&["init", "-q", "-b", "main"]);
    run(&["config", "user.email", "test@example.com"]);
    run(&["config", "user.name", "Test"]);
    std::fs::write(repo.join("seed.txt"), "seed\n").unwrap();
    run(&["add", "-A"]);
    run(&["-c",
          "commit.gpgsign=false",
          "-c",
          "gpg.format=openpgp",
          "commit",
          "-qm",
          "init"]);
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let result = create_agent(&mut store,
                              &json!({"agentId": caller.to_string(), "name": "worker", "agentType": "claude", "repoPath": repo, "createWorktree": true, "branchName": "feature/worker"}),
                              &[]);
    assert_eq!(result.is_error, None);
    let created = store.agents()
                       .iter()
                       .find(|agent| agent.name == "worker")
                       .unwrap();
    assert_ne!(created.folder, repo.to_string_lossy());
    assert!(Path::new(&created.folder).is_dir());
}

#[test]
fn create_agent_missing_fields_without_bench_id_errors() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let result = create_agent(&mut store, &json!({"agentId": caller.to_string()}), &[]);
    assert_eq!(result.is_error, Some(true));
    assert!(result.content[0].text.contains("name"));
    assert!(result.content[0].text.contains("agentType"));
    assert!(result.content[0].text.contains("repoPath"));
}

#[test]
fn close_agent_by_creator_succeeds() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let target = store.create("/tmp/target",
                              CreateOptions { created_by: Some(caller),
                                              insert_after: Some(caller),
                                              ..Default::default() });
    let result = close_agent(&mut store,
                             &json!({"agentId": caller.to_string(), "target": target.to_string()}));
    assert_eq!(result.is_error, None);
    assert!(store.agent(target).is_none());
}

#[test]
fn close_agent_rejects_non_creator() {
    let mut store = AgentStore::new();
    let caller = store.create("/tmp/caller", CreateOptions::default());
    let target = store.create("/tmp/target",
                              CreateOptions { insert_after: Some(caller),
                                              ..Default::default() });
    let result = close_agent(&mut store,
                             &json!({"agentId": caller.to_string(), "target": target.to_string()}));
    assert!(result.content[0].text.contains("Permission denied"));
    assert!(store.agent(target).is_some());
}

#[test]
fn set_status_clears_to_empty() {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/a", CreateOptions::default());
    store.set_status_text(id, "busy".to_string());
    let result = set_status(&mut store,
                            &json!({"agentId": id.to_string(), "status": ""}));
    assert_eq!(result.is_error, None);
    assert_eq!(store.agent(id).unwrap().status_text, "");
    assert_eq!(store.agent(id).unwrap().state,
               knot_agents::AgentState::Idle);
}
