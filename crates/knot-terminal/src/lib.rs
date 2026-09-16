//! Transport-neutral agent terminal sessions.
//!
//! This crate owns command construction and activity forwarding. A later UI or
//! PTY transport can implement [`TerminalTransport`] without changing session
//! lifecycle behavior.

use std::path::Path;
use std::sync::{Arc, Mutex};

mod grid;
mod keys;
mod mouse;
mod pty;

pub use grid::{Cell, Grid, GridEvent, GridSize};
pub use keys::{KeyInput, key_to_bytes};
pub use mouse::{MouseButton, MouseInput, mouse_to_bytes};

use knot_activity::{EventSink, KeyEvent, Tracker, TrackerConfig, tracking_for};
use knot_agent_launch::{
    LaunchRequest, build_agent_command, build_initialization_command, registration_prompt,
    supports_inline_registration,
};
use knot_agents::Agent;
use knot_core::{Persona, Settings};
use thiserror::Error;

pub use pty::PtyTransport;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("terminal transport failed: {0}")]
    Transport(String),
}

pub type Result<T> = std::result::Result<T, TerminalError>;

pub trait TerminalTransport: Send {
    fn send_text(&mut self, text: &str) -> Result<()>;
    fn send_return(&mut self) -> Result<()>;
    fn terminate(&mut self) -> Result<()>;

    /// Resizes the underlying terminal device, if any (a no-op for
    /// transports with no such concept, e.g. tests' `FakeTransport`).
    fn resize(&mut self, _rows: u16, _cols: u16) -> Result<()> {
        Ok(())
    }
}

pub struct SessionConfig<'a> {
    pub settings: &'a Settings,
    pub agent: &'a Agent,
    pub persona: Option<&'a Persona>,
    pub plugin_root: Option<&'a Path>,
}

pub struct SessionPlan {
    pub agent_command: String,
    pub initialization_command: String,
}

impl SessionPlan {
    pub fn build(config: &SessionConfig<'_>) -> Self {
        let request = LaunchRequest {
            agent_type: &config.agent.agent_type,
            agent_id: Some(config.agent.id),
            shell_command: config.agent.shell_command.as_deref(),
            resume_session_id: config.agent.resume_session_id.as_deref(),
            fork_session: config.agent.fork_session,
            persona: config.persona,
            plugin_root: config.plugin_root,
        };
        let agent_command = build_agent_command(config.settings, &request);
        let initialization_command = build_initialization_command(
            &config.agent.folder,
            &agent_command,
            Some(config.agent.id),
        );
        Self {
            agent_command,
            initialization_command,
        }
    }
}

/// Default grid size for a freshly spawned session, before the UI resizes
/// it to match the actual terminal pane (task 2.6 in the terminal-rendering
/// change) - matches the traditional VT100 default.
const DEFAULT_GRID_SIZE: GridSize = GridSize {
    columns: 80,
    rows: 24,
};

pub struct TerminalSession<T> {
    transport: Arc<Mutex<T>>,
    tracker: Arc<Tracker>,
    grid: Option<Arc<Mutex<Grid>>>,
    started: bool,
}

impl<T: TerminalTransport + 'static> TerminalSession<T> {
    pub fn new(config: &SessionConfig<'_>, transport: T, sink: EventSink) -> Self {
        let transport = Arc::new(Mutex::new(transport));
        let tracker = make_tracker(config, Arc::clone(&transport), sink);
        Self {
            transport,
            tracker,
            grid: None,
            started: false,
        }
    }

    /// The session's parsed terminal grid, if it has one - only sessions
    /// spawned via [`Self::spawn_pty`]/[`Self::spawn_pty_with_exit`] do.
    pub fn grid(&self) -> Option<Arc<Mutex<Grid>>> {
        self.grid.clone()
    }

    pub fn spawn_pty<Output>(
        config: &SessionConfig<'_>,
        sink: EventSink,
        on_output: Output,
    ) -> Result<TerminalSession<PtyTransport>>
    where
        Output: Fn(&[u8]) + Send + Sync + 'static,
    {
        Self::spawn_pty_with_exit(config, sink, on_output, |_| {})
    }

    pub fn spawn_pty_with_exit<Output, Exit>(
        config: &SessionConfig<'_>,
        sink: EventSink,
        on_output: Output,
        on_exit: Exit,
    ) -> Result<TerminalSession<PtyTransport>>
    where
        Output: Fn(&[u8]) + Send + Sync + 'static,
        Exit: Fn(Option<i32>) + Send + Sync + 'static,
    {
        let tracker_slot: Arc<Mutex<Option<Arc<Tracker>>>> = Arc::new(Mutex::new(None));
        let output_tracker = Arc::clone(&tracker_slot);
        let exit_tracker = Arc::clone(&tracker_slot);
        let grid = Arc::new(Mutex::new(Grid::new(DEFAULT_GRID_SIZE)));
        let output_grid = Arc::clone(&grid);
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let transport = PtyTransport::spawn(
            &config.agent.folder,
            shell,
            move |bytes| {
                if let Ok(tracker) = output_tracker.lock()
                    && let Some(tracker) = tracker.as_ref()
                {
                    tracker.on_terminal_activity();
                }
                if let Ok(mut grid) = output_grid.lock() {
                    grid.feed(bytes);
                }
                on_output(bytes);
            },
            move |status| {
                if let Ok(tracker) = exit_tracker.lock()
                    && let Some(tracker) = tracker.as_ref()
                {
                    tracker.on_process_exit(status);
                }
                on_exit(status);
            },
        )?;
        let transport = Arc::new(Mutex::new(transport));
        let tracker = make_tracker(config, Arc::clone(&transport), sink);
        *tracker_slot.lock().unwrap() = Some(Arc::clone(&tracker));
        Ok(TerminalSession {
            transport,
            tracker,
            grid: Some(grid),
            started: false,
        })
    }

    pub fn start(&mut self, plan: &SessionPlan) -> Result<()> {
        self.send_text(&plan.initialization_command)?;
        self.transport
            .lock()
            .map_err(|_| TerminalError::Transport("transport lock poisoned".to_string()))?
            .send_return()?;
        self.started = true;
        Ok(())
    }

    pub fn send_text(&mut self, text: &str) -> Result<()> {
        self.transport
            .lock()
            .map_err(|_| TerminalError::Transport("transport lock poisoned".to_string()))?
            .send_text(text)
    }

    pub fn send_command(&mut self, text: &str) -> Result<()> {
        self.send_text(text)?;
        self.transport
            .lock()
            .map_err(|_| TerminalError::Transport("transport lock poisoned".to_string()))?
            .send_return()
    }

    pub fn on_terminal_output(&self) {
        self.tracker.on_terminal_activity();
    }

    pub fn on_user_input(&self, key: KeyEvent) {
        self.tracker.on_user_input(key);
    }

    pub fn on_process_exit(&self, exit_code: Option<i32>) {
        self.tracker.on_process_exit(exit_code);
    }

    /// Resizes the session's grid and, if it has one, the underlying PTY
    /// device to match.
    pub fn resize(&mut self, size: GridSize) -> Result<()> {
        if let Some(grid) = &self.grid
            && let Ok(mut grid) = grid.lock()
        {
            grid.resize(size);
        }
        self.transport
            .lock()
            .map_err(|_| TerminalError::Transport("transport lock poisoned".to_string()))?
            .resize(size.rows as u16, size.columns as u16)
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.tracker.shutdown();
        self.transport
            .lock()
            .map_err(|_| TerminalError::Transport("transport lock poisoned".to_string()))?
            .terminate()?;
        self.started = false;
        Ok(())
    }

    pub fn is_started(&self) -> bool {
        self.started
    }
}

fn make_tracker<T: TerminalTransport + 'static>(
    config: &SessionConfig<'_>,
    transport: Arc<Mutex<T>>,
    mut sink: EventSink,
) -> Arc<Tracker> {
    let mut caller_inject = sink.on_inject_registration.take();
    sink.on_inject_registration = Some(Box::new(move |prompt| {
        if let Ok(mut transport) = transport.lock() {
            let _ = transport.send_text(&prompt);
            let _ = transport.send_return();
        }
        if let Some(caller_inject) = caller_inject.as_mut() {
            caller_inject(prompt);
        }
    }));
    let tracker = Arc::new(Tracker::spawn(
        TrackerConfig {
            agent_type: config.agent.agent_type.clone(),
            is_hook_based: matches!(config.agent.agent_type.as_str(), "claude" | "codex"),
            mcp_enabled: config.settings.mcp_server_enabled,
            inline_registration: supports_inline_registration(&config.agent.agent_type),
            ..TrackerConfig::default()
        },
        tracking_for(&config.agent.agent_type),
        sink,
    ));
    if config.settings.mcp_server_enabled && !supports_inline_registration(&config.agent.agent_type)
    {
        tracker.set_registration_prompt(registration_prompt(config.agent.id));
    }
    tracker
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use knot_activity::ActivitySource;
    use knot_agents::{AgentState, AgentStore, CreateOptions};

    #[derive(Default)]
    struct FakeTransport {
        sent: Vec<String>,
        returns: usize,
        terminated: bool,
    }

    impl TerminalTransport for FakeTransport {
        fn send_text(&mut self, text: &str) -> Result<()> {
            self.sent.push(text.to_string());
            Ok(())
        }

        fn send_return(&mut self) -> Result<()> {
            self.returns += 1;
            Ok(())
        }

        fn terminate(&mut self) -> Result<()> {
            self.terminated = true;
            Ok(())
        }
    }

    fn agent() -> Agent {
        AgentStore::new()
            .agents()
            .first()
            .cloned()
            .unwrap_or_else(|| {
                let mut store = AgentStore::new();
                let id = store.create("/tmp/project", CreateOptions::default());
                store.agent(id).unwrap().clone()
            })
    }

    #[tokio::test]
    async fn session_builds_and_sends_initialization_command() {
        let agent = agent();
        let settings = Settings::default();
        let config = SessionConfig {
            settings: &settings,
            agent: &agent,
            persona: None,
            plugin_root: None,
        };
        let plan = SessionPlan::build(&config);
        let mut session =
            TerminalSession::new(&config, FakeTransport::default(), EventSink::default());

        session.start(&plan).unwrap();

        assert!(session.is_started());
        assert!(plan.initialization_command.contains("/tmp/project"));
    }

    #[tokio::test]
    async fn command_and_lifecycle_events_reach_transport_and_tracker() {
        let agent = agent();
        let settings = Settings::default();
        let config = SessionConfig {
            settings: &settings,
            agent: &agent,
            persona: None,
            plugin_root: None,
        };
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let status_log = Arc::clone(&statuses);
        let sink = EventSink {
            on_status: Some(Box::new(move |event| {
                status_log
                    .lock()
                    .unwrap()
                    .push((event.status, event.source));
            })),
            ..Default::default()
        };
        let mut session = TerminalSession::new(&config, FakeTransport::default(), sink);

        session.send_command("printf ready").unwrap();
        session.on_terminal_output();
        tokio::task::yield_now().await;
        session.on_process_exit(Some(0));
        tokio::task::yield_now().await;
        session.shutdown().unwrap();

        let statuses = statuses.lock().unwrap();
        assert!(statuses.iter().any(|(state, source)| {
            *state == AgentState::Running && *source == ActivitySource::Terminal
        }));
    }

    #[tokio::test]
    async fn pty_session_forwards_raw_output_to_the_caller() {
        let folder = tempfile::tempdir().unwrap();
        let mut agent = agent();
        agent.folder = folder.path().to_string_lossy().into_owned();
        let settings = Settings::default();
        let config = SessionConfig {
            settings: &settings,
            agent: &agent,
            persona: None,
            plugin_root: None,
        };
        let (output_tx, output_rx) = std::sync::mpsc::channel();
        let (exit_tx, exit_rx) = std::sync::mpsc::channel();
        let mut session = TerminalSession::<PtyTransport>::spawn_pty_with_exit(
            &config,
            EventSink::default(),
            move |bytes| {
                let _ = output_tx.send(bytes.to_vec());
            },
            move |status| {
                let _ = exit_tx.send(status);
            },
        )
        .unwrap();

        session
            .start(&SessionPlan {
                agent_command: String::new(),
                initialization_command: "printf ready; exit 0\n".to_string(),
            })
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut output = Vec::new();
        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if let Ok(bytes) = output_rx.recv_timeout(remaining) {
                output.extend(bytes);
                if String::from_utf8_lossy(&output).contains("ready") {
                    break;
                }
            } else {
                break;
            }
        }
        assert!(String::from_utf8_lossy(&output).contains("ready"));
        assert_eq!(
            exit_rx.recv_timeout(std::time::Duration::from_secs(5)),
            Ok(Some(0))
        );
    }

    #[tokio::test]
    async fn pty_session_output_is_reflected_in_its_grid() {
        let folder = tempfile::tempdir().unwrap();
        let mut agent = agent();
        agent.folder = folder.path().to_string_lossy().into_owned();
        let settings = Settings::default();
        let config = SessionConfig {
            settings: &settings,
            agent: &agent,
            persona: None,
            plugin_root: None,
        };
        let mut session =
            TerminalSession::<PtyTransport>::spawn_pty(&config, EventSink::default(), |_| {})
                .unwrap();
        let grid = session.grid().expect("pty session has a grid");

        session
            .start(&SessionPlan {
                agent_command: String::new(),
                initialization_command: "printf ready\n".to_string(),
            })
            .unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if grid.lock().unwrap().row_text(0).contains("ready")
                || (1..24).any(|row| grid.lock().unwrap().row_text(row).contains("ready"))
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "grid never showed the expected output"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }

    #[tokio::test]
    async fn resize_updates_the_grid_and_the_pty() {
        let folder = tempfile::tempdir().unwrap();
        let mut agent = agent();
        agent.folder = folder.path().to_string_lossy().into_owned();
        let settings = Settings::default();
        let config = SessionConfig {
            settings: &settings,
            agent: &agent,
            persona: None,
            plugin_root: None,
        };
        let mut session =
            TerminalSession::<PtyTransport>::spawn_pty(&config, EventSink::default(), |_| {})
                .unwrap();
        let grid = session.grid().expect("pty session has a grid");

        session
            .resize(GridSize {
                columns: 100,
                rows: 40,
            })
            .unwrap();

        assert_eq!(
            grid.lock().unwrap().size(),
            GridSize {
                columns: 100,
                rows: 40
            }
        );

        session
            .start(&SessionPlan {
                agent_command: String::new(),
                initialization_command: "stty size\n".to_string(),
            })
            .unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let rows: Vec<String> = (0..20)
                .map(|row| grid.lock().unwrap().row_text(row))
                .collect();
            if rows.iter().any(|row| row.contains("40 100")) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "pty size was never reported as 40 rows x 100 cols"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }
}
