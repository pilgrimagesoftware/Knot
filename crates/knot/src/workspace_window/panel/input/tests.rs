//! That a Panel-mode agent's declared model and effort options reach the
//! dropdowns `prepare_frame` builds (#451).
//!
//! The options are declared by a real ACP handshake against a fake adapter
//! and published into a real `Ready` slot, then a frame is drawn: the
//! regression read them from a map only Terminal-mode sessions fill, so a
//! test that seeded the options anywhere but the slot the pane draws from
//! would pass against the broken code.

use std::collections::BTreeMap;
use std::sync::Arc;

use gpui_kit::TestAppContext;
use knot_agent_launch::AdapterConfig;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::panel_session;
use crate::settings_global;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::input::EFFORT_SELECTOR_ID;
use crate::workspace_window::panel::input::MODEL_SELECTOR_ID;

/// An adapter that completes the handshake and declares a `model` and an
/// `effort` select option on `session/new`, the way Claude's does.
fn declaring_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
                      id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                      method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                      case "$method" in
                        initialize) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":1,"capabilities":{}}}\n' "$id" ;;
                        session/new) printf '{"jsonrpc":"2.0","id":%s,"result":{"sessionId":"sess-1","configOptions":[{"id":"model","name":"Model","category":"model","type":"select","currentValue":"opus","options":[{"value":"opus","name":"Opus"},{"value":"sonnet","name":"Sonnet"}]},{"id":"effort","name":"Effort","category":"effort","type":"select","currentValue":"high","options":[{"value":"low","name":"Low"},{"value":"high","name":"High"}]}]}}\n' "$id" ;;
                      esac
                    done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// A slot driven to `Ready` against [`declaring_adapter`], on `runtime` so
/// the session's event drain outlives the connect.
fn ready_slot(runtime: &tokio::runtime::Runtime) -> Arc<Mutex<panel_session::PanelSessionSlot>> {
    let (connecting, progress) = panel_session::PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = panel_session::ConnectRequest { config:              &declaring_adapter(),
                                                  cwd:                 "/tmp",
                                                  prior_session_id:    None,
                                                  mcp_url:             None,
                                                  registration_prompt: None,
                                                  session_config:      BTreeMap::new(),
                                                  subagents:           None, };
    runtime.block_on(panel_session::connect_into(&slot, request, &progress, |_| {}));
    assert_eq!(slot.lock().phase(),
               panel_session::PanelPhase::Ready,
               "the fake adapter must connect, or this test says nothing about the selectors");
    slot
}

/// The selected Panel-mode agent's model and effort dropdowns are built
/// from the options its session declared, so the control bar draws them as
/// live selectors rather than as "doesn't report" placeholders.
///
/// The settings are rooted at a temporary directory: opening the window
/// reaches the store, and a `Settings::default()` writes over the user's
/// own roster.
#[gpui_kit::test]
fn a_panel_agents_declared_options_build_its_dropdowns(cx: &mut TestAppContext) {
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    let slot = ready_slot(&runtime);

    let mut store = knot_agents::AgentStore::new();
    let space = knot_core::Workspace { id:        Uuid::new_v4(),
                                       name:      "Only".to_string(),
                                       color_hex: "#123456".to_string(),
                                       agent_ids: Vec::new(), };
    let workspace_id = space.id;
    store.add_workspace(space);
    store.set_current_workspace(workspace_id);
    let agent = store.create("~/agent", knot_agents::CreateOptions::default());
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

    cx.update(|cx| {
          view.update(cx, |view, cx| {
                  view.panel_sessions.insert(agent, slot);
                  view.selected_agent = Some(agent);
                  cx.notify();
              });
      });
    cx.run_until_parked();

    cx.update(|cx| {
          let view = view.read(cx);
          for element_id in [MODEL_SELECTOR_ID, EFFORT_SELECTOR_ID] {
              assert!(view.panel_selectors.contains_key(&(agent, element_id)),
                      "{element_id} must be built from the options the session declared");
          }
      });
}
