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
    /// Whether `session/load` opened this session. False for a fresh
    /// `session/new`, including the fallback taken when a prior session was
    /// named but the adapter does not advertise loading or refused it.
    resumed:    bool,
}

/// Which session to open on a started adapter, and what to open it with.
#[derive(Debug, Clone, Copy, Default)]
pub struct SessionTarget<'a> {
    pub cwd:              &'a str,
    /// The session to `session/load` first, when the adapter can resume.
    pub prior_session_id: Option<&'a str>,
    /// Knot's own MCP HTTP server, when MCP is enabled.
    pub mcp_url:          Option<&'a str>,
    /// The `_meta` the session opens with - the agent's user options, for
    /// an adapter that takes them there.
    pub meta:             Option<&'a serde_json::Value>,
    /// Extra environment for the adapter subprocess - the standing
    /// instructions, for an adapter that reads them from there. Applied on
    /// every spawn, so a resumed session gets them too.
    pub env:              &'a [(String, String)],
    /// Arguments after the adapter's own - the agent's user options, for an
    /// adapter that takes them on its command line.
    pub args:             &'a [String],
}

/// How long to wait for the adapter to answer `initialize` and open a
/// session before failing closed, per design.md's "Panel mode fails
/// closed to Terminal mode with a visible error" requirement - a hung
/// adapter (e.g. blocked on an interactive prompt it can't show over
/// stdio) must not hang the caller forever.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);

impl AcpSession {
    /// Spawns `config`'s adapter and opens `target`'s session, wiring its
    /// MCP URL (Knot's own MCP HTTP server, when MCP is enabled) into the
    /// session through the ACP protocol's own `mcpServers` mechanism and its
    /// `meta` in as the session's `_meta`. If the target names a prior
    /// session and the adapter supports resume, attempts `session/load`
    /// first; on any failure (unsupported or an error response) falls back
    /// to a fresh `session/new` rather than surfacing an error, per the
    /// `agent-lifecycle` layout-restore fallback requirement. Fails with
    /// `AcpError::Timeout` rather than hanging if the adapter never
    /// responds.
    pub async fn start(
        config: &AdapterConfig, target: SessionTarget<'_>, progress: &ConnectProgress)
        -> AcpResult<(Self, Vec<ConfigOption>, mpsc::UnboundedReceiver<SessionEvent>)> {
        Self::start_with_timeout(config, target, progress, CONNECT_TIMEOUT).await
    }

    async fn start_with_timeout(
        config: &AdapterConfig, target: SessionTarget<'_>, progress: &ConnectProgress,
        timeout: Duration)
        -> AcpResult<(Self, Vec<ConfigOption>, mpsc::UnboundedReceiver<SessionEvent>)> {
        tokio::time::timeout(timeout, Self::start_inner(config, target, progress))
            .await
            .unwrap_or(Err(AcpError::Timeout))
    }

    async fn start_inner(
        config: &AdapterConfig, target: SessionTarget<'_>, progress: &ConnectProgress)
        -> AcpResult<(Self, Vec<ConfigOption>, mpsc::UnboundedReceiver<SessionEvent>)> {
        let SessionTarget { cwd,
                            prior_session_id,
                            mcp_url,
                            meta,
                            env,
                            args, } = target;
        let build_command = || {
            let mut command = Command::new(config.command);
            command.args(config.args);
            command.args(args);
            // A Finder-launched macOS app has launchd's minimal PATH, not
            // the user's shell PATH - resolve adapters against the merged
            // path so registered binaries in the standard install
            // locations are found (per `agent-launch-command`'s "ACP
            // launch path" requirement).
            command.env("PATH", adapter_path());
            command.envs(env.iter().map(|(key, value)| (key, value)));
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
        let client = client.with_session_meta(meta.cloned());

        // Gated on what the connected adapter advertises, not on the
        // registry's `supports_resume`: an adapter older than the one the
        // registry was written against may not load, and asking would only
        // cost a round trip before the same fallback.
        let (session, resumed) = match prior_session_id {
            Some(prior) if client.capabilities().supports_resume => {
                report(progress, ConnectStep::Resuming);
                match client.session_load(prior, cwd, mcp_url).await {
                    Ok(session) => (session, true),
                    Err(_) => {
                        report(progress, ConnectStep::OpeningSession);
                        (client.session_new(cwd, mcp_url).await?, false)
                    }
                }
            }
            _ => {
                report(progress, ConnectStep::OpeningSession);
                (client.session_new(cwd, mcp_url).await?, false)
            }
        };

        Ok((Self { client,
                   session_id: session.session_id,
                   resumed },
            session.config_options,
            events))
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Whether this session continues a prior conversation, loaded with
    /// `session/load`, rather than starting a fresh one.
    ///
    /// What decides whether the agent needs registering: asking whether a
    /// prior session was *named* is not enough, since an adapter that cannot
    /// load it opens a fresh session that nothing has registered.
    pub fn resumed(&self) -> bool {
        self.resumed
    }

    /// The adapter subprocess's process id - a panel agent's session root,
    /// for the processes section. `None` once the adapter has been reaped.
    pub fn process_id(&self) -> Option<u32> {
        self.client.process_id()
    }

    /// The connected adapter's declared capabilities (e.g. its supported
    /// permission modes), for panel controls that need to know what the
    /// adapter actually supports before offering a selection.
    pub fn capabilities(&self) -> &knot_acp::AgentCapabilities {
        self.client.capabilities()
    }

    /// The connected agent's name and version, as it reported them on
    /// `initialize`, for showing the user which adapter and version a
    /// session is running. `None` when the agent did not report them.
    pub fn agent_info(&self) -> Option<&knot_acp::AgentInfo> {
        self.client.agent_info()
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

mod adapter_update;

pub use adapter_update::{AdapterUpdateStatus, check_adapter_update, update_adapter};

#[cfg(test)]
mod tests;
