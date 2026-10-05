//! A session over a recording transport, and over a real shell.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use gpui_terminal::GridSize;
use knot_activity::ActivitySource;
use knot_agents::{Agent, AgentState, AgentStore, CreateOptions};
use knot_core::Settings;
use parking_lot::Mutex;

use super::*;

/// Failsafe for the tests that drive a real PTY subprocess. Deliberately far
/// longer than the work normally takes (milliseconds), because its only job
/// is to fail a genuinely hung test instead of blocking forever. It is not a
/// latency assertion.
///
/// A tight budget here is exactly what made these tests flaky: `cargo test`
/// runs them in parallel with the rest of the workspace compiling, and
/// spawning a shell through a PTY on a saturated machine can take seconds.
const PTY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Default)]
struct Log {
    written:    Vec<u8>,
    terminated: bool,
}

struct FakeTransport {
    log: Arc<Mutex<Log>>,
}

impl Transport for FakeTransport {
    fn write(&mut self, bytes: &[u8]) -> gpui_terminal::Result<()> {
        self.log.lock().written.extend_from_slice(bytes);
        Ok(())
    }

    fn resize(&mut self, _size: GridSize) -> gpui_terminal::Result<()> {
        Ok(())
    }

    fn terminate(&mut self) -> gpui_terminal::Result<()> {
        self.log.lock().terminated = true;
        Ok(())
    }
}

fn agent() -> Agent {
    let mut store = AgentStore::new();
    let id = store.create("/tmp/project", CreateOptions::default());
    store.agent(id).cloned().expect("the agent just created")
}

fn config<'a>(settings: &'a Settings, agent: &'a Agent) -> SessionConfig<'a> {
    SessionConfig { settings,
                    agent,
                    persona: None,
                    plugin_root: None }
}

/// An agent whose folder is a fresh temp dir, kept alive by the guard.
fn agent_in_temp_dir() -> (Agent, tempfile::TempDir) {
    let folder = tempfile::tempdir().expect("a temp dir");
    let mut agent = agent();
    agent.folder = folder.path().to_string_lossy().into_owned();
    (agent, folder)
}

#[tokio::test]
async fn start_sends_the_initialization_command_and_its_return() {
    let agent = agent();
    let settings = Settings::default();
    let config = config(&settings, &agent);
    let plan = SessionPlan::build(&config);
    let log = Arc::new(Mutex::new(Log::default()));
    let mut session = TerminalSession::new(&config,
                                           FakeTransport { log: Arc::clone(&log), },
                                           EventSink::default());

    session.start(&plan).unwrap();

    assert!(session.is_started());
    assert!(plan.initialization_command.contains("/tmp/project"));
    assert_eq!(log.lock().written,
               format!("{}\r", plan.initialization_command).into_bytes());
}

#[tokio::test]
async fn command_and_lifecycle_events_reach_transport_and_tracker() {
    // `AgentStore` always coerces a non-shell agent to Panel view mode (it
    // never gets a `TerminalSession` in the running app - see
    // `ensure_session`), but the tracker still supports terminal-output
    // tracking for that combination; force it here to exercise that branch
    // of `tracking_for`.
    let mut agent = agent();
    agent.view_mode = knot_core::ViewMode::Terminal;
    let settings = Settings::default();
    let config = config(&settings, &agent);
    let statuses = Arc::new(Mutex::new(Vec::new()));
    let status_log = Arc::clone(&statuses);
    let sink = EventSink { on_status: Some(Box::new(move |event| {
                                               status_log.lock().push((event.status, event.source));
                                           })),
                           ..Default::default() };
    let log = Arc::new(Mutex::new(Log::default()));
    let mut session = TerminalSession::new(&config, FakeTransport { log: Arc::clone(&log), }, sink);

    session.send_command("printf ready").unwrap();
    session.on_terminal_output();
    tokio::task::yield_now().await;
    session.on_process_exit(Some(0));
    tokio::task::yield_now().await;
    session.shutdown().unwrap();

    assert_eq!(log.lock().written, b"printf ready\r");
    assert!(log.lock().terminated);
    let statuses = statuses.lock();
    assert!(statuses.iter().any(|(state, source)| {
                               *state == AgentState::Running && *source == ActivitySource::Terminal
                           }));
}

#[tokio::test]
async fn pty_session_forwards_output_and_exit_to_the_caller() {
    let (agent, _folder) = agent_in_temp_dir();
    let settings = Settings::default();
    let config = config(&settings, &agent);
    let (output_tx, output_rx) = mpsc::channel();
    let (exit_tx, exit_rx) = mpsc::channel();
    let mut session = TerminalSession::spawn_pty(&config,
                                                 EventSink::default(),
                                                 move |bytes| {
                                                     let _ = output_tx.send(bytes.to_vec());
                                                 },
                                                 move |report| {
                                                     let _ = exit_tx.send(report.code);
                                                 }).unwrap();

    session.start(&SessionPlan { agent_command:          String::new(),
                                 initialization_command: "printf ready; exit 0".to_string(), })
           .unwrap();

    assert_eq!(exit_rx.recv_timeout(PTY_TIMEOUT), Ok(Some(0)));
    let output = output_rx.try_iter().flatten().collect::<Vec<_>>();
    assert!(String::from_utf8_lossy(&output).contains("ready"));
}

#[tokio::test]
async fn pty_session_output_is_reflected_in_its_grid() {
    let (agent, _folder) = agent_in_temp_dir();
    let settings = Settings::default();
    let config = config(&settings, &agent);
    let mut session =
        TerminalSession::spawn_pty(&config, EventSink::default(), |_| {}, |_| {}).unwrap();

    session.start(&SessionPlan { agent_command:          String::new(),
                                 initialization_command: "printf ready".to_string(), })
           .unwrap();

    let deadline = Instant::now() + PTY_TIMEOUT;
    while !session.terminal().with_grid(|grid| {
                                 (0..grid.size().rows).any(|row| {
                                                          grid.row_text(row).contains("ready")
                                                      })
                             })
    {
        assert!(Instant::now() < deadline,
                "grid never showed the expected output");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The scan rides the output hook, ahead of the parse - so a URL is found
/// however the grid wraps or scrolls it.
#[tokio::test]
async fn pty_session_output_is_scanned_for_pull_requests() {
    let (agent, _folder) = agent_in_temp_dir();
    let settings = Settings::default();
    let config = config(&settings, &agent);
    let mut session =
        TerminalSession::spawn_pty(&config, EventSink::default(), |_| {}, |_| {}).unwrap();

    session.start(&SessionPlan { agent_command:          String::new(),
                                 initialization_command:
                                     "printf 'https://github.com/acme/widget/pull/42\\n'"
                                         .to_string(), })
           .unwrap();

    let deadline = Instant::now() + PTY_TIMEOUT;
    let mut urls = Vec::new();
    while urls.is_empty() {
        assert!(Instant::now() < deadline,
                "the pull request URL was never noted");
        tokio::time::sleep(Duration::from_millis(20)).await;
        urls = session.take_pull_request_urls();
    }
    assert_eq!(urls, ["https://github.com/acme/widget/pull/42"]);
}
