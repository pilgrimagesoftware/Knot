//! Restarting agents in a real workspace window (`agent-lifecycle`:
//! "Restart"): keeping the conversation leaves the ACP session id in place
//! for the reconnect to load, starting a new one clears it, and either way
//! the old session is torn down.
//!
//! The agents use an agent type with no terminal process and no ACP
//! adapter, so nothing launches. Settings are rooted at a temporary
//! directory; restarting persists the roster.

use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "restart-test-inert";

struct Fixture {
    window: VisualTestContext,
    view:   Entity<WorkspaceWindow>,
    store:  Arc<Mutex<knot_agents::AgentStore>>,
    agents: Vec<Uuid>,
    _dir:   TempDir,
}

/// A workspace window over `count` agents that each have a live-looking
/// conversation: a session id and an ACP session id.
fn window_with_conversations(count: usize, cx: &mut TestAppContext) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    let agents: Vec<Uuid> = (0..count).map(|index| {
                                          let id = store.create(format!("~/agent-{index}"),
                                  knot_agents::CreateOptions { agent_type:
                                                                   Some(INERT_AGENT_TYPE.into()),
                                                               workspace_id: Some(workspace_id),
                                                               ..Default::default() });
                                          store.set_session_id(id, format!("session-{index}"));
                                          store.set_acp_session_id(id, format!("acp-{index}"));
                                          id
                                      })
                                      .collect();
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let view = cx.update(|cx| {
                   gpui_kit::init(cx);
                   WindowRegistry::install(cx);
                   crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()),
                                                     cx);
                   WorkspaceWindow::open(Arc::clone(&store), messages, workspace_id, cx);
                   WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
               });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    Fixture { window: VisualTestContext::from_window(handle, cx),
              view,
              store,
              agents,
              _dir: dir }
}

impl Fixture {
    fn restart(&mut self, ids: &[Uuid], keep_conversation: bool) {
        let ids = ids.to_vec();
        self.view.update(&mut self.window, |view, cx| {
                     view.restart_agents(&ids, keep_conversation, cx);
                 });
    }

    fn acp_session(&self, id: Uuid) -> Option<String> {
        self.store
            .lock()
            .agent(id)
            .and_then(|agent| agent.acp_session_id.clone())
    }

    fn restart_token(&self, id: Uuid) -> Uuid {
        self.store
            .lock()
            .agent(id)
            .expect("the agent exists")
            .restart_token
    }
}

#[gpui_kit::test]
fn keeping_the_conversation_leaves_the_acp_session_to_load(cx: &mut TestAppContext) {
    let mut fixture = window_with_conversations(1, cx);
    let id = fixture.agents[0];
    let token = fixture.restart_token(id);

    fixture.restart(&[id], true);

    assert_eq!(fixture.acp_session(id).as_deref(), Some("acp-0"));
    assert_ne!(fixture.restart_token(id),
               token,
               "the old session is still torn down");
}

#[gpui_kit::test]
fn a_new_conversation_clears_the_acp_session(cx: &mut TestAppContext) {
    let mut fixture = window_with_conversations(1, cx);
    let id = fixture.agents[0];

    fixture.restart(&[id], false);

    assert_eq!(fixture.acp_session(id), None);
    assert_eq!(fixture.store
                      .lock()
                      .agent(id)
                      .and_then(|agent| agent.session_id.clone()),
               None);
}

#[gpui_kit::test]
fn restart_all_applies_the_choice_to_every_agent(cx: &mut TestAppContext) {
    let mut fixture = window_with_conversations(3, cx);
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.restart_all_agents(true, cx)
                });
    for (index, id) in fixture.agents.clone().into_iter().enumerate() {
        assert_eq!(fixture.acp_session(id), Some(format!("acp-{index}")));
    }

    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.restart_all_agents(false, cx)
                });
    for id in fixture.agents.clone() {
        assert_eq!(fixture.acp_session(id), None);
    }
}
