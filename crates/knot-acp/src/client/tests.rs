//! Unit tests for [`super`].

use super::*;
use crate::error::SessionEndCause;

/// A fake agent running `script` under `sh`.
///
/// Six fixtures built this the same way and differed only in the script,
/// which is the only part worth reading in any of them.
fn sh_agent(script: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new("sh");
    command.arg("-c").arg(script);
    command
}

/// A fake agent handling `initialize` (with a configurable protocol
/// version and resume capability) and `session/new`/`session/load`.
fn fake_agent(protocol_version: u32, supports_resume: bool) -> Command {
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{protocol_version},\"agentCapabilities\":{{\"loadSession\":{supports_resume}}}}}}}" ;;
            session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#
    );
    sh_agent(script)
}

/// A fake agent whose `session/prompt` answers with a stop reason and
/// sends no `turn_end` notification - which is how ACP actually ends a
/// turn.
fn prompting_agent() -> Command {
    let script = format!(
                         r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{}}}}}}" ;;
            session/prompt) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"stopReason\":\"end_turn\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#
    );
    sh_agent(script)
}

/// The turn-end event has to be synthesized from the `session/prompt`
/// response, because ACP has no `turn_end` session update. Without it a
/// caller that gates input on "a turn is in flight" - the panel's Send
/// button and Enter key both do - stays blocked forever after the very
/// first prompt.
#[tokio::test]
async fn a_finished_prompt_emits_a_turn_end_with_the_responses_stop_reason() {
    let (client, mut events) = AcpClient::connect(prompting_agent()).await
                                                                    .expect("connect");

    client.session_prompt("sess-1", "hello")
          .await
          .expect("prompt");

    let event = events.recv().await.expect("a turn-end event");
    assert!(matches!(&event,
                     SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason })
                     if stop_reason == "end_turn"),
            "expected a turn end, got {event:?}");
}

/// A prompt that fails must still end the turn, or the same gate wedges
/// the panel permanently on one bad request.
#[tokio::test]
async fn a_failed_prompt_still_ends_the_turn() {
    let (client, mut events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");
    drop(client.close());

    let _ = client.session_prompt("sess-1", "hello").await;

    let event = events.recv().await.expect("a turn-end event");
    assert!(matches!(&event, SessionEvent::Update(SessionUpdate::TurnEnd { .. })),
            "expected a turn end even on failure, got {event:?}");
}

#[tokio::test]
async fn connect_negotiates_matching_protocol_version() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");

    assert!(client.capabilities().supports_resume);
}

#[tokio::test]
async fn connect_fails_closed_on_unsupported_protocol_version() {
    let result = AcpClient::connect(fake_agent(PROTOCOL_VERSION + 1, false)).await;

    assert!(matches!(
                result,
                Err(AcpError::UnsupportedProtocolVersion { agent, client }) if agent == PROTOCOL_VERSION + 1 && client == PROTOCOL_VERSION
            ));
}

#[tokio::test]
async fn session_new_returns_session_id() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, true)).await
                                                              .expect("connect");

    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session id");

    assert_eq!(session.session_id, "sess-1");
}

/// A fake agent that appends every line it receives to `log_path`, so
/// the test can inspect the raw `session/new`/`session/load` params it
/// was sent. `declares_http` controls whether its `initialize`
/// response advertises `mcpCapabilities.http` - required before an
/// `http`-type `mcpServers` entry is valid per the ACP spec.
fn logging_fake_agent(log_path: &std::path::Path, declares_http: bool) -> Command {
    let mcp_capabilities = if declares_http {
        r#"{\"http\":true}"#
    }
    else {
        r#"{}"#
    };
    let script = format!(
                         r#"while IFS= read -r line; do
          echo "$line" >> '{}'
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"protocolVersion\":{PROTOCOL_VERSION},\"agentCapabilities\":{{\"mcpCapabilities\":{mcp_capabilities}}}}}}}" ;;
            session/new) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{\"sessionId\":\"sess-1\"}}}}" ;;
            *) echo "{{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{{}}}}" ;;
          esac
        done"#,
                         log_path.display()
    );
    sh_agent(script)
}

#[tokio::test]
async fn session_new_carries_the_knot_mcp_server_when_enabled_and_supported() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), true)).await
                                                                .expect("connect");
    client.session_new("/tmp/project", Some("http://127.0.0.1:8767/mcp"))
          .await
          .expect("session");

    let log = std::fs::read_to_string(log.path()).unwrap();
    let request_line = log.lines()
                          .find(|line| line.contains("session/new"))
                          .expect("session/new request logged");
    assert!(request_line.contains(r#""type":"http""#));
    assert!(request_line.contains(r#""name":"knot""#));
    assert!(request_line.contains(r#""url":"http://127.0.0.1:8767/mcp""#));
    assert!(request_line.contains(r#""headers":[]"#));
}

#[tokio::test]
async fn session_new_omits_the_mcp_server_when_the_agent_does_not_support_http() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), false)).await
                                                                 .expect("connect");
    client.session_new("/tmp/project", Some("http://127.0.0.1:8767/mcp"))
          .await
          .expect("session");

    let log = std::fs::read_to_string(log.path()).unwrap();
    let request_line = log.lines()
                          .find(|line| line.contains("session/new"))
                          .expect("session/new request logged");
    assert!(request_line.contains(r#""mcpServers":[]"#));
}

#[tokio::test]
async fn session_new_sends_no_mcp_servers_when_disabled() {
    let log = tempfile::NamedTempFile::new().unwrap();
    let (client, _events) =
        AcpClient::connect(logging_fake_agent(log.path(), true)).await
                                                                .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session");

    let log = std::fs::read_to_string(log.path()).unwrap();
    let request_line = log.lines()
                          .find(|line| line.contains("session/new"))
                          .expect("session/new request logged");
    assert!(request_line.contains(r#""mcpServers":[]"#));
}

#[tokio::test]
async fn session_load_fails_closed_when_resume_unsupported() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, false)).await
                                                               .expect("connect");

    let result = client.session_load("sess-1", "/tmp/project", None).await;

    assert!(matches!(result, Err(AcpError::ResumeNotSupported)));
}

#[tokio::test]
async fn subprocess_exit_ends_session_and_resolves_pending_request_with_error() {
    // A subprocess that answers `initialize` then exits immediately,
    // simulating a crash mid-turn: the next request must resolve with
    // an error rather than hang.
    let command = sh_agent(
                           r#"read -r line
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}""#,
    );
    let (client, mut events) = AcpClient::connect(command).await.expect("connect");

    let result = client.session_new("/tmp/project", None).await;

    assert!(result.is_err());
    let ended = events.recv().await.expect("ended event");
    assert!(matches!(ended,
                     SessionEvent::Ended(SessionEndCause::ProcessExited { .. })));
}

/// Fake agent that, once `session/new` succeeds, streams a text delta
/// and a turn-end update, then sends a `session/request_permission`
/// request; on receiving the client's response it echoes the chosen
/// option back as one more text delta, so the test can assert the
/// decision actually reached the agent.
fn permission_flow_agent() -> Command {
    sh_agent(
             r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
              ;;
            session/new)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}"
              echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"hello\"}}"
              echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"turn_end\",\"stopReason\":\"end_turn\"}}"
              echo "{\"jsonrpc\":\"2.0\",\"id\":100,\"method\":\"session/request_permission\",\"params\":{\"toolCall\":{\"toolCallId\":\"tc1\"},\"options\":[{\"optionId\":\"allow-once\",\"name\":\"Allow\"},{\"optionId\":\"deny\",\"name\":\"Deny\"}]}}"
              ;;
            "")
              pid=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
              if [ "$pid" = "100" ]; then
                opt=$(echo "$line" | sed -E 's/.*"optionId":"([^"]+)".*/\1/')
                echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"decision:$opt\"}}"
              fi
              ;;
          esac
        done"#,
    )
}

#[tokio::test]
async fn close_while_permission_pending_ends_session_without_hanging() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");
    let _text = events.recv().await.expect("text delta");
    let _turn_end = events.recv().await.expect("turn end");
    let _permission = events.recv().await.expect("permission request");

    client.close().await;

    // Per the "session closed while a permission request is pending"
    // scenario, the still-pending decision auto-resolves to Deny
    // (rather than hanging) as part of tearing the session down.
    let ended = events.recv().await.expect("ended event");
    assert!(matches!(ended, SessionEvent::Ended(_)));
}

#[tokio::test]
async fn ordered_updates_stream_text_and_turn_end() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");

    let first = events.recv().await.expect("first update");
    let second = events.recv().await.expect("second update");

    assert!(matches!(first, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "hello"));
    assert!(matches!(second, SessionEvent::Update(SessionUpdate::TurnEnd { stop_reason }) if stop_reason == "end_turn"));
}

#[tokio::test]
async fn permission_deny_decision_is_delivered_to_the_agent() {
    let (client, mut events) = AcpClient::connect(permission_flow_agent()).await
                                                                          .expect("connect");
    client.session_new("/tmp/project", None)
          .await
          .expect("session id");
    let _text = events.recv().await.expect("text delta");
    let _turn_end = events.recv().await.expect("turn end");
    let permission = match events.recv().await.expect("permission request") {
        SessionEvent::PermissionRequest(request) => request,
        other => panic!("expected a permission request, got {other:?}"),
    };

    client.answer_permission(&permission, PermissionDecision::Deny);

    let confirmation = events.recv().await.expect("decision echoed back");
    assert!(matches!(confirmation, SessionEvent::Update(SessionUpdate::TextDelta { text }) if text == "decision:deny"));
}

/// Fake agent declaring one `select` Session Config Option ("mode") on
/// `session/new`, and answering `session/set_config_option` with an
/// updated `currentValue`.
fn config_options_agent() -> Command {
    sh_agent(
             r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
              ;;
            session/new)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\",\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"category\":\"mode\",\"type\":\"select\",\"currentValue\":\"ask\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
              ;;
            session/set_config_option)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"category\":\"mode\",\"type\":\"select\",\"currentValue\":\"code\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
              ;;
            *) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}" ;;
          esac
        done"#,
    )
}

#[tokio::test]
async fn session_new_parses_declared_config_options() {
    let (client, _events) = AcpClient::connect(config_options_agent()).await
                                                                      .expect("connect");

    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session");

    assert_eq!(session.config_options.len(), 1);
    assert_eq!(session.config_options[0].id, "mode");
    assert_eq!(session.config_options[0].category.as_deref(), Some("mode"));
    assert_eq!(session.config_options[0].options.len(), 2);
}

/// The same fake agent, minus the `category` field - which ACP makes
/// optional: "Clients MUST handle missing or unknown categories
/// gracefully"
/// (<https://agentclientprotocol.com/protocol/v2/session-config-options>).
///
/// The categorized fixture above is the reason issue #194 went unnoticed
/// for so long: it hard-codes `"category":"mode"`, so every test saw a
/// roster Knot could match while real agents that omitted the field lost
/// all three selectors and the prompt's risk colour.
fn uncategorized_config_options_agent() -> Command {
    sh_agent(
             r#"while IFS= read -r line; do
          id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
          method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
          case "$method" in
            initialize)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}"
              ;;
            session/new)
              echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\",\"configOptions\":[{\"id\":\"mode\",\"name\":\"Session Mode\",\"type\":\"select\",\"currentValue\":\"ask\",\"options\":[{\"value\":\"ask\",\"name\":\"Ask\"},{\"value\":\"code\",\"name\":\"Code\"}]}]}}"
              ;;
            *) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}" ;;
          esac
        done"#,
    )
}

/// An agent that omits `category` must still parse, and must reach the
/// client with `category: None` rather than being dropped or defaulted to
/// something that would match by accident.
#[tokio::test]
async fn session_new_accepts_config_options_without_a_category() {
    let (client, _events) =
        AcpClient::connect(uncategorized_config_options_agent()).await
                                                                .expect("connect");

    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session");

    assert_eq!(session.config_options.len(), 1);
    assert_eq!(session.config_options[0].id, "mode");
    assert_eq!(session.config_options[0].category, None,
               "a missing category must stay missing, not acquire a default");
    assert_eq!(session.config_options[0].kind, "select");
    assert_eq!(session.config_options[0].options.len(), 2);
}

#[tokio::test]
async fn set_config_option_sends_the_selection_and_returns_the_updated_list() {
    let (client, _events) = AcpClient::connect(config_options_agent()).await
                                                                      .expect("connect");
    let session = client.session_new("/tmp/project", None)
                        .await
                        .expect("session");

    let updated = client.session_set_config_option(&session.session_id, "mode", "code")
                        .await
                        .expect("set config option");

    assert_eq!(updated[0].current_value, serde_json::json!("code"));
}

/// The panel agent's session root, reachable from the client because the
/// transport module is private. See
/// `openspec/specs/agent-processes/spec.md` -- "Every running agent exposes
/// a session root process".
#[tokio::test]
async fn a_connected_client_reports_the_adapters_pid() {
    let (client, _events) =
        AcpClient::connect(fake_agent(PROTOCOL_VERSION, false)).await
                                                               .expect("connect");

    let pid = client.process_id()
                    .expect("a live adapter has a process id");

    assert!(pid > 1, "got {pid}");
    assert_ne!(pid, std::process::id());
}
