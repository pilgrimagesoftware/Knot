//! Benching and deploying through a real workspace window: that the entry is
//! written before the agent closes, that a failed write keeps it, and that a
//! deployed entry lands in this workspace or, when stale, is pruned with a
//! message.
//!
//! Every test roots its settings in a temporary directory: the window
//! persists the roster, and a `Settings::default()` writes over the user's
//! own.

use std::sync::Arc;

use gpui_kit::{Entity, TestAppContext};
use knot_core::{BenchAgent, StartupPrompt};
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::settings_global;
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;

struct Fixture {
    view:      Entity<WorkspaceWindow>,
    store:     Arc<Mutex<knot_agents::AgentStore>>,
    workspace: Uuid,
    agent:     Uuid,
    folder:    TempDir,
    _dir:      TempDir,
}

/// A window over one workspace holding one agent in a real folder. With
/// `broken_store`, the settings root is a file, so every settings write
/// fails.
fn open_window(cx: &mut TestAppContext, broken_store: bool) -> Fixture {
    let folder = TempDir::new().expect("an agent folder");
    let mut store = knot_agents::AgentStore::new();
    let space = knot_core::Workspace { id:        Uuid::new_v4(),
                                       name:      "Only".to_string(),
                                       color_hex: "#123456".to_string(),
                                       agent_ids: Vec::new(), };
    let workspace = space.id;
    store.add_workspace(space);
    store.set_current_workspace(workspace);
    let agent = store.create(folder.path().to_string_lossy().into_owned(),
                        knot_agents::CreateOptions { name: Some("Worker".into()),
                                                     startup_prompt: StartupPrompt::custom("go"),
                                                     workspace_id: Some(workspace),
                                                     ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let root = if broken_store {
        let file = dir.path().join("not-a-directory");
        std::fs::write(&file, "").unwrap();
        file
    }
    else {
        dir.path().to_path_buf()
    };
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          settings_global::install(knot_core::Settings::with_store_root(&root), cx);
          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), workspace, cx);
      });
    let view = cx.update(|cx| WindowRegistry::workspace_view(WindowKey::Workspace(workspace), cx))
                 .expect("opening a workspace registers its view");
    Fixture { view,
              store,
              workspace,
              agent,
              folder,
              _dir: dir }
}

#[gpui_kit::test]
fn benching_saves_the_agent_then_closes_it(cx: &mut TestAppContext) {
    let fixture = open_window(cx, false);
    let agent = fixture.agent;

    cx.update(|cx| {
          fixture.view
                 .update(cx, |view, cx| view.bench_agent(agent, cx))
                 .expect("the bench write succeeds");
          let bench = settings_global::read(cx).bench_agents.clone();
          assert_eq!(bench.len(), 1);
          assert_eq!(bench[0].name, "Worker");
          assert_eq!(bench[0].startup_prompt, StartupPrompt::custom("go"));
      });
    assert!(fixture.store.lock().agent(agent).is_none());
}

#[gpui_kit::test]
fn a_failed_bench_write_keeps_the_agent(cx: &mut TestAppContext) {
    let fixture = open_window(cx, true);
    let agent = fixture.agent;

    cx.update(|cx| {
          let result = fixture.view
                              .update(cx, |view, cx| view.bench_agent(agent, cx));
          assert!(result.is_err());
      });
    assert!(fixture.store.lock().agent(agent).is_some(),
            "an agent whose bench entry was not written must not be closed");
}

#[gpui_kit::test]
fn benching_an_owner_closes_its_companion_and_benches_only_the_owner(cx: &mut TestAppContext) {
    let fixture = open_window(cx, false);
    let agent = fixture.agent;
    let companion = fixture.store.lock().create_shell_companion(agent).unwrap();

    cx.update(|cx| {
          fixture.view
                 .update(cx, |view, cx| view.bench_agent(agent, cx))
                 .expect("the bench write succeeds");
          assert_eq!(settings_global::read(cx).bench_agents.len(), 1);
      });
    let store = fixture.store.lock();
    assert!(store.agent(agent).is_none() && store.agent(companion).is_none());
}

#[gpui_kit::test]
fn deploying_an_entry_creates_and_selects_it_in_this_workspace(cx: &mut TestAppContext) {
    let fixture = open_window(cx, false);
    let mut entry = BenchAgent::new(Uuid::new_v4(),
                                    "From bench",
                                    None,
                                    fixture.folder.path().to_string_lossy());
    entry.startup_prompt = StartupPrompt::custom("hello");

    cx.update(|cx| {
          fixture.view
                 .update(cx, |view, cx| view.deploy_bench_entry(&entry, cx));
      });

    let created = cx.update(|cx| fixture.view.read(cx).selected_agent)
                    .expect("the deployed agent is selected");
    let store = fixture.store.lock();
    let agent = store.agent(created).unwrap();
    assert_eq!(agent.name, "From bench");
    assert_eq!(agent.startup_prompt, StartupPrompt::custom("hello"));
    assert!(store.workspaces()
                 .iter()
                 .find(|workspace| workspace.id == fixture.workspace)
                 .is_some_and(|workspace| workspace.agent_ids.contains(&created)));
}

#[gpui_kit::test]
fn a_stale_entry_is_pruned_and_reported(cx: &mut TestAppContext) {
    let fixture = open_window(cx, false);
    let entry = BenchAgent::new(Uuid::new_v4(), "Gone", None, "/definitely/not/here");
    cx.update(|cx| {
          settings_global::write_persisting(cx, |settings| settings.add_bench_agent(entry.clone()))
              .unwrap();
      });
    let before = fixture.store.lock().agents().len();

    cx.update(|cx| {
          fixture.view
                 .update(cx, |view, cx| view.deploy_bench_entry(&entry, cx));
          assert!(settings_global::read(cx).bench_agents.is_empty());
          let error = fixture.view
                             .read(cx)
                             .error
                             .clone()
                             .expect("a message names the entry");
          assert!(error.contains("Gone") && error.contains("/definitely/not/here"),
                  "{error}");
      });
    assert_eq!(fixture.store.lock().agents().len(), before);
}
