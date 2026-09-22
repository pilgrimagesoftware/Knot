//! An ACP-backed agent session (Panel mode), independent of
//! [`crate::TerminalSession`] - switching an agent's view mode
//! starts/stops one of these without disturbing the other, per
//! `openspec/specs/acp-panel-ui/spec.md`'s "Switch to Terminal mid-turn"
//! scenario.

use std::sync::Arc;
use std::time::Duration;

use knot_acp::{
    AcpClient, AcpError, ConfigOption, PermissionDecision, PermissionRequest, Result as AcpResult,
    SessionEvent,
};
use knot_agent_launch::{AdapterConfig, InstallMethod, adapter_path};
use parking_lot::Mutex;
use tokio::process::Command;
use tokio::sync::mpsc;

/// What a connection attempt is currently doing, so the caller can show
/// progress instead of an unchanging "Connecting…" through a spawn, a
/// package install, and a session handshake - steps that between them can
/// take tens of seconds.
///
/// `Copy` and `Eq` on purpose: the UI detects progress by comparing the
/// step it last drew against the current one, so a step must be cheap to
/// snapshot and compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectStep {
    /// Spawning the adapter and negotiating capabilities with it.
    Starting { program: &'static str },
    /// Running the adapter's declared install command, because the binary
    /// wasn't on `PATH`. Much the slowest step when it happens.
    Installing { program: &'static str },
    /// Attempting `session/load` for a previously saved session.
    Resuming,
    /// Opening a fresh session with `session/new`.
    OpeningSession,
}

impl ConnectStep {
    /// A sentence for the connecting placeholder, in the caller's voice.
    pub fn label(self) -> String {
        match self {
            Self::Starting { program } => format!("Starting {program}…"),
            Self::Installing { program } => format!("Installing {program}…"),
            Self::Resuming => "Resuming the previous session…".to_string(),
            Self::OpeningSession => "Opening a session…".to_string(),
        }
    }
}

/// A connection attempt's current [`ConnectStep`], shared between the
/// background connect task and whatever is rendering its progress.
pub type ConnectProgress = Arc<Mutex<ConnectStep>>;

fn report(progress: &ConnectProgress, step: ConnectStep) {
    {
        let mut current = progress.lock();
        *current = step;
    }
}

/// A live ACP connection for one agent: the adapter subprocess plus its
/// open session id. Cheap to clone - see `knot_acp::AcpClient`'s doc
/// comment.
#[derive(Clone)]
pub struct AcpSession {
    client:     AcpClient,
    session_id: String,
}

/// How long to wait for the adapter to answer `initialize` and open a
/// session before failing closed, per design.md's "Panel mode fails
/// closed to Terminal mode with a visible error" requirement - a hung
/// adapter (e.g. blocked on an interactive prompt it can't show over
/// stdio) must not hang the caller forever.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

impl AcpSession {
    /// Spawns `config`'s adapter and opens a session for `cwd`, wiring
    /// `mcp_url` (Knot's own MCP HTTP server, when MCP is enabled) into the
    /// session through the ACP protocol's own `mcpServers` mechanism. If
    /// `prior_session_id` is given and the adapter supports resume,
    /// attempts `session/load` first; on any failure (unsupported or an
    /// error response) falls back to a fresh `session/new` rather than
    /// surfacing an error, per the `agent-lifecycle` layout-restore
    /// fallback requirement. Fails with `AcpError::Timeout` rather than
    /// hanging if the adapter never responds.
    pub async fn start(
        config: &AdapterConfig, cwd: &str, prior_session_id: Option<&str>, mcp_url: Option<&str>,
        progress: &ConnectProgress)
        -> AcpResult<(Self, Vec<ConfigOption>, mpsc::UnboundedReceiver<SessionEvent>)> {
        Self::start_with_timeout(config,
                                 cwd,
                                 prior_session_id,
                                 mcp_url,
                                 progress,
                                 CONNECT_TIMEOUT).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn start_with_timeout(
        config: &AdapterConfig, cwd: &str, prior_session_id: Option<&str>, mcp_url: Option<&str>,
        progress: &ConnectProgress, timeout: Duration)
        -> AcpResult<(Self, Vec<ConfigOption>, mpsc::UnboundedReceiver<SessionEvent>)> {
        tokio::time::timeout(
            timeout,
            Self::start_inner(config, cwd, prior_session_id, mcp_url, progress),
        )
        .await
        .unwrap_or(Err(AcpError::Timeout))
    }

    async fn start_inner(
        config: &AdapterConfig, cwd: &str, prior_session_id: Option<&str>, mcp_url: Option<&str>,
        progress: &ConnectProgress)
        -> AcpResult<(Self, Vec<ConfigOption>, mpsc::UnboundedReceiver<SessionEvent>)> {
        let build_command = || {
            let mut command = Command::new(config.command);
            command.args(config.args);
            // A Finder-launched macOS app has launchd's minimal PATH, not
            // the user's shell PATH - resolve adapters against the merged
            // path so registered binaries in the standard install
            // locations are found (per `agent-launch-command`'s "ACP
            // launch path" requirement).
            command.env("PATH", adapter_path());
            command
        };

        report(progress, ConnectStep::Starting { program: config.command, });
        let (client, events) = match AcpClient::connect(build_command()).await {
            Err(AcpError::Spawn(error))
                if error.kind() == std::io::ErrorKind::NotFound
                   && let Some(install) = config.install =>
            {
                // Auto-install, silently, on first use - per design.md
                // decision 6, the adapter is Knot-published configuration
                // (not arbitrary user-supplied code), so this doesn't ask
                // for confirmation. Adapters with no declared install
                // method (`install: None`) fall through to the original
                // spawn error unchanged.
                report(progress,
                       ConnectStep::Installing { program: install.command, });
                run_install(install).await?;
                report(progress, ConnectStep::Starting { program: config.command, });
                AcpClient::connect(build_command()).await?
            }
            other => other?,
        };

        let session = match prior_session_id {
            Some(prior) if client.capabilities().supports_resume => {
                report(progress, ConnectStep::Resuming);
                match client.session_load(prior, cwd, mcp_url).await {
                    Ok(session) => session,
                    Err(_) => {
                        report(progress, ConnectStep::OpeningSession);
                        client.session_new(cwd, mcp_url).await?
                    }
                }
            }
            _ => {
                report(progress, ConnectStep::OpeningSession);
                client.session_new(cwd, mcp_url).await?
            }
        };

        Ok((Self { client,
                   session_id: session.session_id },
            session.config_options,
            events))
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// The connected adapter's declared capabilities (e.g. its supported
    /// permission modes), for panel controls that need to know what the
    /// adapter actually supports before offering a selection.
    pub fn capabilities(&self) -> &knot_acp::AgentCapabilities {
        self.client.capabilities()
    }

    pub async fn prompt(&self, text: &str) -> AcpResult<()> {
        self.client.session_prompt(&self.session_id, text).await
    }

    /// Applies one Session Config Option selection (mode, model, reasoning
    /// effort, ...) and returns the agent's updated full list.
    pub async fn set_config_option(&self, config_id: &str, value: &str)
                                   -> AcpResult<Vec<ConfigOption>> {
        self.client
            .session_set_config_option(&self.session_id, config_id, value)
            .await
    }

    pub async fn cancel(&self) -> AcpResult<()> {
        self.client.session_cancel(&self.session_id).await
    }

    /// Answers a pending `session/request_permission` request surfaced via
    /// a `SessionEvent::PermissionRequest` from this session's event
    /// stream.
    pub fn answer_permission(&self, request: &PermissionRequest, decision: PermissionDecision) {
        self.client.answer_permission(request, decision);
    }

    /// Closes the ACP connection. Never touches any terminal transport -
    /// the two are independent per-agent objects.
    pub async fn stop(&self) {
        self.client.close().await;
    }
}

/// Runs an adapter's declared install command to completion. Per
/// design.md decision 6, this is invoked silently (no confirmation
/// prompt) - the adapter is Knot-published configuration, not arbitrary
/// user-supplied code.
async fn run_install(install: InstallMethod) -> AcpResult<()> {
    let output = Command::new(install.command).args(install.args)
                                              // The package manager (e.g. `npm`) can itself live in a standard
                                              // directory the GUI PATH misses - see the adapter spawn above.
                                              .env("PATH", adapter_path())
                                              .output()
                                              .await
                                              .map_err(|error| {
                                                  AcpError::InstallFailed(format!(
            "failed to run `{} {}`: {error}",
            install.command,
            install.args.join(" ")
        ))
                                              })?;
    if !output.status.success() {
        return Err(AcpError::InstallFailed(format!(
            "`{} {}` exited with {}: {}",
            install.command,
            install.args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use knot_agent_launch::{AdapterConfig, InstallMethod, adapter_path};
    use parking_lot::Mutex;

    use super::*;
    use crate::{TerminalError, TerminalTransport};

    /// A throwaway progress cell for tests that don't assert on progress.
    fn no_progress() -> ConnectProgress {
        Arc::new(Mutex::new(ConnectStep::Starting { program: "test" }))
    }

    #[derive(Default)]
    struct FakeTransport {
        sent: Arc<Mutex<Vec<String>>>,
    }

    impl TerminalTransport for FakeTransport {
        fn send_text(&mut self, text: &str) -> Result<(), TerminalError> {
            self.sent.lock().push(text.to_string());
            Ok(())
        }

        fn send_return(&mut self) -> Result<(), TerminalError> {
            Ok(())
        }

        fn terminate(&mut self) -> Result<(), TerminalError> {
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
            AcpSession::start(&path_reporting_adapter_launch(),
                              "/tmp/project",
                              None,
                              None,
                              &no_progress()).await
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
            AcpSession::start(&fake_adapter_launch(),
                              "/tmp/project",
                              None,
                              None,
                              &no_progress()).await
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
                                                    "/tmp/project",
                                                    None,
                                                    None,
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
                                                                   args:    install_args, }) };

        let (session, _config_options, _events) =
            AcpSession::start(&launch, "/tmp/project", None, None, &no_progress())
                .await
                .expect("connect after auto-install");

        assert_eq!(session.session_id(), "sess-installed");
        assert!(bin_path.exists(),
                "install step should have created the adapter binary");
    }
}
