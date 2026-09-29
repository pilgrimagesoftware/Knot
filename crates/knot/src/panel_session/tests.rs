//! Connecting a panel session: when the slot goes live, and which first
//! turns a connection sends.

use knot_agent_launch::AdapterConfig;

use super::*;

/// A fake adapter that completes the handshake but never answers
/// `session/prompt` - standing in for an agent whose first turn is
/// blocked (Gemini stalls its registration turn on a
/// `session/request_permission` for the first Knot MCP tool call,
/// which only a `Ready` slot can show and answer).
fn stalling_prompt_adapter() -> AdapterConfig {
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

/// The registration turn must not gate the slot: an agent that asks
/// permission mid-registration can only be answered through a `Ready`
/// slot, so publishing the handle after the turn finishes deadlocks
/// the connection.
#[tokio::test]
async fn the_slot_goes_ready_before_the_registration_turn_finishes() {
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = ConnectRequest { config:              &stalling_prompt_adapter(),
                                   cwd:                 "/tmp/project",
                                   prior_session_id:    None,
                                   mcp_url:             None,
                                   registration_prompt: Some("register".to_string()),
                                   session_config:      BTreeMap::new(),
                                   default_mode:        None,
                                   option_mode:         None,
                                   session_meta:        None,
                                   env:                 Vec::new(),
                                   args:                Vec::new(),
                                   subagents:           None,
                                   startup_prompt:      None, };

    let watched = Arc::clone(&slot);
    let became_ready = async move {
        for _ in 0..200 {
            let phase = watched.lock().phase();
            if phase == PanelPhase::Ready {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    };

    tokio::select! {
        ready = became_ready => assert!(
            ready,
            "the slot must reach Ready while the registration turn is still in flight"
        ),
        () = connect_into(&slot, request, &progress, |_| {}) => {
            panic!("the stalled registration turn should never finish")
        }
    }
}

/// A fake adapter that advertises `loadSession`, logs every method it is
/// sent to `$KNOT_TEST_LOG`, and loads or refuses a prior session as
/// `$KNOT_TEST_LOAD` says. Its load result is empty, the shape `codex-acp`
/// 2.0.0 answers with.
fn logging_loading_adapter() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  echo "$method" >> "$KNOT_TEST_LOG"
  case "$method" in
    initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{\"loadSession\":true}}}" ;;
    session/load)
      if [ "$KNOT_TEST_LOAD" = refuse ]; then
        echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"error\":{\"code\":-32602,\"message\":\"no such session\"}}"
      else
        echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}"
      fi
      ;;
    session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-new\"}}" ;;
    session/prompt) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"stopReason\":\"end_turn\"}}" ;;
  esac
done"#,
    ],
                    supports_resume:           true,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// Connects to [`logging_loading_adapter`] naming prior session `thread-7`,
/// with a registration prompt ready, and returns the methods it was sent
/// and the session id the caller was handed to persist.
async fn connect_naming_a_prior_session(load: &str) -> (Vec<String>, String) {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let log = dir.path().join("methods.log");
    let (connecting, progress) = PanelSessionSlot::connecting();
    let slot = Arc::new(Mutex::new(connecting));
    let request = ConnectRequest { config:              &logging_loading_adapter(),
                                   cwd:                 "/tmp/project",
                                   prior_session_id:    Some("thread-7"),
                                   mcp_url:             None,
                                   registration_prompt: Some("register".to_string()),
                                   session_config:      BTreeMap::new(),
                                   default_mode:        None,
                                   option_mode:         None,
                                   session_meta:        None,
                                   env:                 vec![("KNOT_TEST_LOG".to_string(),
                                                              log.display().to_string()),
                                                             ("KNOT_TEST_LOAD".to_string(),
                                                              load.to_string())],
                                   args:                Vec::new(),
                                   subagents:           None,
                                   startup_prompt:      None, };
    let mut persisted = String::new();
    connect_into(&slot, request, &progress, |id| persisted = id.to_string()).await;
    if let PanelSessionSlot::Ready(handle) = &*slot.lock() {
        let session = handle.session();
        tokio::spawn(async move { session.stop().await });
    }
    let methods = std::fs::read_to_string(&log).unwrap_or_default()
                                               .lines()
                                               .map(str::to_string)
                                               .collect();
    (methods, persisted)
}

/// A resumed agent is already registered, so its first turn is not the
/// registration request - and the id persisted for the next launch is the
/// loaded one, so the conversation stays resumable.
#[tokio::test]
async fn a_loaded_session_sends_no_registration_turn() {
    let (methods, persisted) = connect_naming_a_prior_session("load").await;

    assert!(methods.contains(&"session/load".to_string()), "{methods:?}");
    assert!(!methods.contains(&"session/new".to_string()), "{methods:?}");
    assert!(!methods.contains(&"session/prompt".to_string()),
            "a resumed session is not re-registered: {methods:?}");
    assert_eq!(persisted, "thread-7");
}

/// The case the prior-session id alone got wrong: the adapter could not
/// load it, so the session is fresh and must be registered like any other.
#[tokio::test]
async fn a_refused_load_registers_the_fresh_session_it_falls_back_to() {
    let (methods, persisted) = connect_naming_a_prior_session("refuse").await;

    assert!(methods.contains(&"session/new".to_string()), "{methods:?}");
    assert!(methods.contains(&"session/prompt".to_string()),
            "the fallback session is sent the registration turn: {methods:?}");
    assert_eq!(persisted, "sess-new");
}
