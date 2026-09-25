//! That a fresh session's startup prompt is expanded, queued once behind a
//! registration turn that is still running, and delivered only when that
//! turn ends - for an agent nobody has selected.
//!
//! The session is a real ACP handshake against a fake adapter that never
//! answers `session/prompt`, so the registration turn stays in flight for as
//! long as the test needs it to. The settings are rooted at a temporary
//! directory: opening the window reaches the store, and a
//! `Settings::default()` writes over the user's own roster.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{Entity, TestAppContext};
use knot_agent_launch::{AdapterConfig, ContextSource};
use knot_core::StartupPrompt;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session::{self, PanelPhase, PanelSessionSlot, StartupRequest};
use crate::settings_global;
use crate::window_registry::{WindowKey, WindowRegistry};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

/// Completes the handshake and never answers anything else, so the first
/// turn never ends.
fn stalling_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
                      id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                      method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                      case "$method" in
                        initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
                        session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}" ;;
                      esac
                    done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// A slot that reaches `Ready` with its registration turn still running and
/// a startup prompt of `text` expanded for an agent named "Knot 3".
fn registering_slot(runtime: &tokio::runtime::Runtime, text: &str) -> Arc<Mutex<PanelSessionSlot>> {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let startup = StartupRequest { text:    text.to_string(),
                                   context: ContextSource { agent_name: "Knot 3".into(),
                                                            folder: "/nonexistent".into(),
                                                            ..Default::default() }, };
    let connecting = Arc::clone(&slot);
    runtime.spawn(async move {
               let config = stalling_adapter();
               let request = panel_session::ConnectRequest { config:              &config,
                                                             cwd:                 "/tmp",
                                                             prior_session_id:    None,
                                                             mcp_url:             None,
                                                             registration_prompt:
                                                                 Some("register".into()),
                                                             session_config:      BTreeMap::new(),
                                                             subagents:           None,
                                                             startup_prompt:      Some(startup), };
               panel_session::connect_into(&connecting, request, &progress, |_| {}).await;
           });
    for _ in 0..500 {
        if slot.lock().phase() == PanelPhase::Ready {
            return slot;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the fake adapter never connected");
}

struct Fixture {
    view:      Entity<WorkspaceWindow>,
    agent:     Uuid,
    /// A second agent, selected in place of `agent`.
    other:     Uuid,
    workspace: String,
    _dir:      TempDir,
}

fn open_window(cx: &mut TestAppContext, startup_prompt: Option<StartupPrompt>) -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = knot_core::Workspace { id:        Uuid::new_v4(),
                                       name:      "Only".to_string(),
                                       color_hex: "#123456".to_string(),
                                       agent_ids: Vec::new(), };
    let workspace_id = space.id;
    store.add_workspace(space);
    store.set_current_workspace(workspace_id);
    let agent = store.create("/tmp/agent",
                             knot_agents::CreateOptions { startup_prompt,
                                                          workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let other = store.create("/tmp/other",
                             knot_agents::CreateOptions { workspace_id: Some(workspace_id),
                                                          ..Default::default() });
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), workspace_id, cx);
      });
    let view =
        cx.update(|cx| WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx))
          .expect("opening a workspace registers its view");
    Fixture { view,
              agent,
              other,
              workspace: "Only".to_string(),
              _dir: dir }
}

#[gpui_kit::test]
fn the_startup_prompt_waits_behind_the_registration_turn(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = registering_slot(&runtime, "I am {{agent.name}}");
    let fixture = open_window(cx, None);
    let (agent, other) = (fixture.agent, fixture.other);

    cx.update(|cx| {
          fixture.view.update(cx, |view, _| {
                          view.selected_agent = Some(other);
                          view.panel_sessions.insert(agent, Arc::clone(&slot));
                          assert!(view.queue_startup_prompts());
                          assert!(!view.queue_startup_prompts(),
                                  "a handle gives its startup prompt up once");
                          let queue = &view.panel_prompt_queues[&agent];
                          assert_eq!(queue.len(), 1);
                          assert_eq!(queue[0].text, "I am Knot 3");
                          assert_eq!(queue[0].origin, PromptOrigin::Startup);

                          assert!(!view.drain_panel_prompt(agent),
                                  "the registration turn is still running");
                          assert!(!view.panel_prompt_queues[&agent][0].in_flight);
                      });
      });

    // The registration turn ends.
    if let PanelSessionSlot::Ready(handle) = &*slot.lock() {
        handle.state().lock().turn_active = false;
    }
    cx.update(|cx| {
          fixture.view.update(cx, |view, _| {
                          assert_eq!(view.selected_agent,
                                     Some(other),
                                     "delivery must not wait for the agent to be selected");
                          assert!(view.drain_panel_prompt(agent));
                          assert!(view.panel_prompt_queues[&agent][0].in_flight);
                      });
      });
}

#[gpui_kit::test]
fn a_startup_request_resolves_the_library_and_names_the_workspace(cx: &mut TestAppContext) {
    let fixture = open_window(cx, StartupPrompt::custom("in {{workspace}}"));
    let agent = fixture.agent;

    cx.update(|cx| {
          let view = fixture.view.read(cx);
          let agent = view.store.lock().agent(agent).cloned().unwrap();
          let request = view.startup_request(&agent, cx)
                            .expect("a custom prompt resolves");
          assert_eq!(request.text, "in {{workspace}}");
          assert_eq!(request.context.workspace.as_deref(),
                     Some(fixture.workspace.as_str()));

          let mut dangling = agent.clone();
          dangling.startup_prompt = Some(StartupPrompt::Library(Uuid::new_v4()));
          assert!(view.startup_request(&dangling, cx).is_none());
          let mut none = agent;
          none.startup_prompt = None;
          assert!(view.startup_request(&none, cx).is_none());
      });
}
