//! The startup prompt through the store: deployed from the bench in the form
//! the entry holds it, persisted, and edited without a restart.

use knot_core::{BenchAgent, StartupPrompt};

use super::super::*;

#[test]
fn deploying_carries_the_startup_prompt_in_its_stored_form() {
    let mut store = AgentStore::default();
    let prompt = Uuid::new_v4();
    let mut bench = BenchAgent::new(Uuid::new_v4(), "Worker", None, "/repo");
    bench.startup_prompt = Some(StartupPrompt::Library(prompt));

    let first = store.deploy_bench(&bench, None, |_| true)
                     .expect("folder exists");
    let second = store.deploy_bench(&bench, None, |_| true)
                      .expect("folder exists");

    assert_ne!(first, second, "one entry deploys any number of times");
    for id in [first, second] {
        assert_eq!(store.agent(id).unwrap().startup_prompt,
                   Some(StartupPrompt::Library(prompt)));
    }
}

#[test]
fn the_startup_prompt_survives_saving_and_loading() {
    let mut store = AgentStore::default();
    let id = store.create("/repo",
                          CreateOptions { startup_prompt: StartupPrompt::custom("go"),
                                          ..Default::default() });

    let saved = store.saved_agents(false);

    assert_eq!(saved[0].startup_prompt, StartupPrompt::custom("go"));
    assert_eq!(crate::convert::from_saved(&saved[0]).startup_prompt,
               store.agent(id).unwrap().startup_prompt);
}

/// `agent-lifecycle` - "A new startup prompt waits for the next fresh
/// session".
#[test]
fn editing_only_the_startup_prompt_does_not_restart() {
    let mut store = AgentStore::default();
    let id = store.create("/repo", CreateOptions::default());
    store.agent_mut(id).unwrap().state = AgentState::Running;
    let agent = store.agent(id).unwrap();
    let token = agent.restart_token;
    let req = EditRequest { name: agent.name.clone(),
                            avatar: agent.avatar.clone(),
                            folder: Some(agent.folder.clone()),
                            agent_type: Some(agent.agent_type.clone()),
                            startup_prompt: StartupPrompt::custom("next time"),
                            ..Default::default() };

    store.edit(id, req).expect("edit succeeds");

    let agent = store.agent(id).unwrap();
    assert_eq!(agent.restart_token, token);
    assert_eq!(agent.state, AgentState::Running);
    assert_eq!(agent.startup_prompt, StartupPrompt::custom("next time"));
}
