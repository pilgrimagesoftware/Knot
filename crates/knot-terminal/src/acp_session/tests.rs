//! Starting an ACP session: the adapter subprocess it spawns, and which
//! session it opens.

use std::sync::Arc;

use gpui_terminal::{GridSize, Transport};
use knot_agent_launch::{AdapterConfig, InstallMethod, adapter_path};
use parking_lot::Mutex;

use super::*;

/// A throwaway progress cell for tests that don't assert on progress.
fn no_progress() -> ConnectProgress {
    Arc::new(Mutex::new(ConnectStep::Starting { program: "test" }))
}

/// A fresh session in the tests' project folder, with nothing else set.
fn project() -> SessionTarget<'static> {
    SessionTarget { cwd: "/tmp/project",
                    ..SessionTarget::default() }
}

/// A fake adapter that names its session for whether `session/new`
/// arrived with a `_meta`.
fn meta_reporting_adapter_launch() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
session/new)
  case "$line" in
    *'"_meta":{"claudeCode"'*) name=sess-meta ;;
    *) name=sess-plain ;;
  esac
  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"$name\"}}"
  ;;
  esac
done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// A fake adapter that advertises `loadSession` and answers `session/load`
/// the way `codex-acp` 2.0.0 does - an empty result, no `sessionId`.
fn loading_adapter_launch() -> AdapterConfig {
    AdapterConfig { args: &[
                            "-c",
                            r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{\"loadSession\":true}}}" ;;
session/load) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{}}" ;;
session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-new\"}}" ;;
  esac
done"#,
    ],
                    ..fake_adapter_launch() }
}

/// Advertises `loadSession` and then refuses the load, as an adapter
/// does for a session it no longer has.
fn refusing_adapter_launch() -> AdapterConfig {
    AdapterConfig { args: &[
                            "-c",
                            r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{\"loadSession\":true}}}" ;;
session/load) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"error\":{\"code\":-32602,\"message\":\"no such session\"}}" ;;
session/new) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-new\"}}" ;;
  esac
done"#,
    ],
                    ..fake_adapter_launch() }
}

fn resuming(prior: &'static str) -> SessionTarget<'static> {
    SessionTarget { prior_session_id: Some(prior),
                    ..project() }
}

#[tokio::test]
async fn a_loaded_session_is_resumed_under_its_own_id() {
    let (session, _options, _events) = AcpSession::start(&loading_adapter_launch(),
                                                         resuming("thread-7"),
                                                         &no_progress()).await
                                                                        .expect("connect");
    assert!(session.resumed());
    assert_eq!(session.session_id(), "thread-7");
    session.stop().await;
}

/// An older adapter that does not advertise loading: the prior session is
/// not asked for, and the fresh one is not a resume.
#[tokio::test]
async fn a_prior_session_on_an_adapter_that_cannot_load_is_not_a_resume() {
    let (session, _options, _events) = AcpSession::start(&fake_adapter_launch(),
                                                         resuming("thread-7"),
                                                         &no_progress()).await
                                                                        .expect("connect");
    assert!(!session.resumed());
    assert_eq!(session.session_id(), "sess-1");
    session.stop().await;
}

#[tokio::test]
async fn a_refused_load_falls_back_to_a_fresh_session_that_is_not_a_resume() {
    let (session, _options, _events) = AcpSession::start(&refusing_adapter_launch(),
                                                         resuming("thread-7"),
                                                         &no_progress()).await
                                                                        .expect("connect");
    assert!(!session.resumed());
    assert_eq!(session.session_id(), "sess-new");
    session.stop().await;
}

/// The agent's own MCP URL (`?agent=<id>`, #544) has to reach both the
/// load and the fresh session a refused load falls back to - a fallback
/// on the bare URL would leave the agent unable to call knot tools as
/// itself.
#[tokio::test]
async fn the_mcp_url_reaches_the_load_and_its_fallback() {
    let dir = tempfile::TempDir::new().expect("a temporary directory");
    let log = dir.path().join("requests.log");
    let launch = AdapterConfig { args: &[
                                         "-c",
                                         r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"agentCapabilities\":{\"loadSession\":true,\"mcpCapabilities\":{\"http\":true}}}}" ;;
session/load)
  echo "$line" >> "$KNOT_TEST_LOG"
  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"error\":{\"code\":-32602,\"message\":\"no such session\"}}"
  ;;
session/new)
  echo "$line" >> "$KNOT_TEST_LOG"
  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-new\"}}"
  ;;
  esac
done"#,
    ],
                                 ..fake_adapter_launch() };
    let env = [("KNOT_TEST_LOG".to_owned(), log.display().to_string())];
    let url = "http://127.0.0.1:9/mcp?agent=agent-7";
    let target = SessionTarget { prior_session_id: Some("thread-7"),
                                 mcp_url: Some(url),
                                 env: &env,
                                 ..project() };

    let (session, _options, _events) =
        AcpSession::start(&launch, target, &no_progress()).await
                                                          .expect("connect");
    session.stop().await;

    let requests = std::fs::read_to_string(&log).expect("the adapter logged its requests");
    let lines: Vec<&str> = requests.lines().collect();
    assert_eq!(lines.len(), 2, "a load, then its fallback: {requests}");
    assert!(lines[0].contains("session/load") && lines[0].contains(url),
            "{}",
            lines[0]);
    assert!(lines[1].contains("session/new") && lines[1].contains(url),
            "{}",
            lines[1]);
}

#[tokio::test]
async fn a_fresh_session_is_not_a_resume() {
    let (session, _options, _events) =
        AcpSession::start(&loading_adapter_launch(), project(), &no_progress()).await
                                                                               .expect("connect");
    assert!(!session.resumed());
    session.stop().await;
}

#[tokio::test]
async fn the_target_meta_opens_the_session() {
    let meta = serde_json::json!({ "claudeCode": { "options": { "extraArgs": {} } } });
    let target = SessionTarget { meta: Some(&meta),
                                 ..project() };
    let (session, _options, _events) = AcpSession::start(&meta_reporting_adapter_launch(),
                                                         target,
                                                         &no_progress()).await
                                                                        .expect("connect");
    assert_eq!(session.session_id(), "sess-meta");
    session.stop().await;

    let (session, _options, _events) = AcpSession::start(&meta_reporting_adapter_launch(),
                                                         project(),
                                                         &no_progress()).await
                                                                        .expect("connect");
    assert_eq!(session.session_id(), "sess-plain");
    session.stop().await;
}

#[derive(Default)]
struct FakeTransport {
    sent: Arc<Mutex<Vec<String>>>,
}

impl Transport for FakeTransport {
    fn write(&mut self, bytes: &[u8]) -> gpui_terminal::Result<()> {
        self.sent
            .lock()
            .push(String::from_utf8_lossy(bytes).into_owned());
        Ok(())
    }

    fn resize(&mut self, _size: GridSize) -> gpui_terminal::Result<()> {
        Ok(())
    }

    fn terminate(&mut self) -> gpui_terminal::Result<()> {
        Ok(())
    }
}

/// A fake adapter: answers `initialize`/`session/new`, then streams one
/// `session/update` text delta.
fn fake_adapter_launch() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
                      id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
                      method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
                      case "$method" in
                        initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
                        session/new)
                          echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-1\"}}"
                          echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"hi\"}}"
                          ;;
                      esac
                    done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

/// A fake adapter that streams the child process's `$PATH` back as the
/// `session/new` text delta - so a test can assert what `PATH` the
/// adapter subprocess actually launched with, without the JSON-RPC
/// framing getting in the way.
fn path_reporting_adapter_launch() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &[
                                                 "-c",
                                                 r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  method=$(echo "$line" | sed -nE 's/.*"method":"([^"]+)".*/\1/p')
  case "$method" in
initialize) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
session/new)
  echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-path\"}}"
  echo "{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{\"sessionUpdate\":\"text_delta\",\"text\":\"$PATH\"}}"
  ;;
  esac
done"#,
    ],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

#[tokio::test]
async fn the_adapter_subprocess_is_spawned_with_the_merged_adapter_path() {
    let (session, _config_options, mut events) =
        AcpSession::start(&path_reporting_adapter_launch(), project(), &no_progress())
            .await
            .expect("connect");
    assert_eq!(session.session_id(), "sess-path");

    let update = events.recv().await.expect("session update");
    match update {
        SessionEvent::Update(knot_acp::SessionUpdate::TextDelta { text }) => {
            // The merged path's fallback dirs are appended after any
            // process `PATH` entries, so last-five covers exactly them.
            let merged = adapter_path();
            assert!(!merged.is_empty(), "adapter_path built an empty PATH");
            for dir in merged.split(':').rev().take(5) {
                assert!(!dir.is_empty() && text.contains(dir),
                        "adapter subprocess saw PATH `{text}`; expected it to contain `{dir}`");
            }
        }
        other => panic!("expected a text delta, got {other:?}"),
    }

    session.stop().await;
}

/// The standing instructions reach some adapters only through their
/// environment, so what the target names has to be what the subprocess
/// sees.
#[tokio::test]
async fn the_target_env_reaches_the_adapter_subprocess() {
    let launch = AdapterConfig { args: &[
                                         "-c",
                                         r#"while IFS= read -r line; do
  id=$(echo "$line" | sed -E 's/.*"id":([0-9]+).*/\1/')
  case "$line" in
*'"initialize"'*) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"protocolVersion\":1,\"capabilities\":{}}}" ;;
*'"session/new"'*) echo "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"sessionId\":\"sess-$KNOT_TEST_ENV\"}}" ;;
  esac
done"#,
    ],
                                 ..path_reporting_adapter_launch() };
    let env = [("KNOT_TEST_ENV".to_owned(), "carried".to_owned())];
    let target = SessionTarget { env: &env,
                                 ..project() };
    let (session, _options, _events) =
        AcpSession::start(&launch, target, &no_progress()).await
                                                          .expect("connect");
    assert_eq!(session.session_id(), "sess-carried");
    session.stop().await;
}

#[tokio::test]
async fn starting_and_stopping_an_acp_session_never_touches_the_terminal_transport() {
    use knot_agents::{Agent, AgentStore, CreateOptions};
    use knot_core::Settings;

    use crate::{SessionConfig, TerminalSession};

    let mut store = AgentStore::new();
    let agent_id = store.create("/tmp/project", CreateOptions::default());
    let agent: Agent = store.agent(agent_id).unwrap().clone();
    let settings = Settings::default();
    let config = SessionConfig { settings:    &settings,
                                 agent:       &agent,
                                 persona:     None,
                                 plugin_root: None, };
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut terminal_session = TerminalSession::new(&config,
                                                    FakeTransport { sent: Arc::clone(&sent), },
                                                    knot_activity::EventSink::default());
    terminal_session.send_text("terminal is alive").unwrap();

    let (session, _config_options, mut events) =
        AcpSession::start(&fake_adapter_launch(), project(), &no_progress()).await
                                                                            .expect("connect");
    assert_eq!(session.session_id(), "sess-1");

    // The ACP update the fake adapter streamed right after session/new
    // is already in flight - receiving it after the "switch back to
    // Terminal" (stop()) proves it still lands rather than being
    // dropped by the switch.
    let update = events.recv().await.expect("session update");
    assert!(matches!(
                update,
                SessionEvent::Update(knot_acp::SessionUpdate::TextDelta { text }) if text == "hi"
            ));

    session.stop().await;

    assert_eq!(*sent.lock(),
               vec!["terminal is alive".to_string()],
               "the terminal transport must be untouched by the ACP session's lifecycle");
}

/// A fake adapter that never answers anything - simulates a hung
/// process (e.g. blocked on an interactive prompt it can't show over
/// stdio, as `gemini --acp` does without `--skip-trust`).
fn hanging_adapter_launch() -> AdapterConfig {
    AdapterConfig { command:                   "sh",
                    args:                      &["-c", "while true; do sleep 1; done"],
                    supports_resume:           false,
                    supports_permission_modes: false,
                    install:                   None, }
}

#[tokio::test]
async fn start_fails_closed_with_a_visible_error_instead_of_hanging() {
    let result = AcpSession::start_with_timeout(&hanging_adapter_launch(),
                                                project(),
                                                &no_progress(),
                                                std::time::Duration::from_millis(50)).await;

    assert!(matches!(result, Err(AcpError::Timeout)));
}

#[tokio::test]
async fn missing_adapter_is_auto_installed_and_the_connection_is_retried() {
    let dir = tempfile::tempdir().unwrap();
    let bin_path = dir.path().join("fake-acp-adapter");
    let bin_path_string = bin_path.to_string_lossy().into_owned();
    assert!(!bin_path.exists(), "the adapter binary must not exist yet");

    // The "install" step writes a responder script to `bin_path` and
    // makes it executable, standing in for a real `npm install -g`.
    let install_script = format!("cat > '{bin_path_string}' <<'SCRIPT'\n#!/bin/sh\nwhile IFS= read -r line; do\n  id=$(echo \"$line\" | sed -E 's/.*\"id\":([0-9]+).*/\\1/')\n  method=$(echo \"$line\" | sed -nE 's/.*\"method\":\"([^\"]+)\".*/\\1/p')\n  case \"$method\" in\n    initialize) echo \"{{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":$id,\\\"result\\\":{{\\\"protocolVersion\\\":1,\\\"capabilities\\\":{{}}}}}}\" ;;\n    session/new) echo \"{{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":$id,\\\"result\\\":{{\\\"sessionId\\\":\\\"sess-installed\\\"}}}}\" ;;\n  esac\ndone\nSCRIPT\nchmod +x '{bin_path_string}'");

    let command: &'static str = Box::leak(bin_path_string.clone().into_boxed_str());
    let install_args: &'static [&'static str] = Box::leak(vec![
        "-c",
        Box::leak(install_script.into_boxed_str()) as &'static str,
    ].into_boxed_slice());
    let launch = AdapterConfig { command,
                                 args: &[],
                                 supports_resume: false,
                                 supports_permission_modes: false,
                                 install: Some(InstallMethod { command: "sh",
                                                               args:    install_args,
                                                               package: "test-package", }) };

    let (session, _config_options, _events) =
        AcpSession::start(&launch, project(), &no_progress()).await
                                                             .expect("connect after auto-install");

    assert_eq!(session.session_id(), "sess-installed");
    assert!(bin_path.exists(),
            "install step should have created the adapter binary");
}
