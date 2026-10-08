//! A shell agent's terminal: a `gpui_terminal::Terminal` plus the activity
//! tracker that reads its output, and the pull request scan beside it.
//!
//! The grid, the transport and the input encodings live in `gpui-terminal`;
//! what is Knot's is here - which shell runs where, what the tracker sees,
//! and the registration prompt it may inject.

use std::sync::Arc;

use gpui_terminal::{ExitReport, PtyTransport, Terminal, TerminalBuilder, Transport};
use knot_activity::{EventSink, KeyEvent, Tracker, TrackerConfig, tracking_for};
use knot_agent_launch::{registration_prompt, supports_inline_registration};
use parking_lot::Mutex;

use crate::pull_requests::PullRequestTap;
use crate::{Result, SessionConfig, SessionPlan};

pub struct TerminalSession<T> {
    terminal:      Terminal<T>,
    tracker:       Arc<Tracker>,
    pull_requests: Arc<Mutex<PullRequestTap>>,
    started:       bool,
}

impl<T: Transport> TerminalSession<T> {
    /// A session over a transport that is already running, with nothing
    /// watching its output - what a test double gets.
    pub fn new(config: &SessionConfig<'_>, transport: T, sink: EventSink) -> Self {
        let terminal =
            TerminalBuilder::new().connect(|_| Ok::<_, std::convert::Infallible>(transport))
                                  .unwrap_or_else(|never| match never {});
        let tracker = make_tracker(config, terminal.clone(), sink);
        Self { terminal,
               tracker,
               pull_requests: Arc::default(),
               started: false }
    }

    /// The terminal, for the view that draws it. Clones share it.
    pub fn terminal(&self) -> &Terminal<T> {
        &self.terminal
    }

    /// The session root: the process the transport spawned, while it runs.
    ///
    /// What the processes section enumerates descendants of. `None` once the
    /// child has exited, and for a transport that spawns nothing.
    pub fn process_id(&self) -> Option<u32> {
        self.terminal.process_id()
    }

    pub fn start(&mut self, plan: &SessionPlan) -> Result<()> {
        self.send_command(&plan.initialization_command)?;
        self.started = true;
        Ok(())
    }

    pub fn send_text(&mut self, text: &str) -> Result<()> {
        Ok(self.terminal.write(text.as_bytes())?)
    }

    pub fn send_command(&mut self, text: &str) -> Result<()> {
        send_command(&self.terminal, text)
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

    /// Take the pull request URLs seen in the output since the last call.
    /// See [`crate::pull_requests`].
    pub fn take_pull_request_urls(&self) -> Vec<String> {
        self.pull_requests.lock().take()
    }

    /// Stops the tracker and ends the shell.
    ///
    /// A shell that already exited is left alone: killing it again fails with
    /// "No such process", which made every exit-driven agent removal log a
    /// failed shutdown. A shell that exits between the check and the kill
    /// still reports that error; it is the same harmless race, now narrow.
    pub fn shutdown(&mut self) -> Result<()> {
        self.tracker.shutdown();
        if self.terminal.exit_report().is_none() {
            self.terminal.terminate()?;
        }
        self.started = false;
        Ok(())
    }

    pub fn is_started(&self) -> bool {
        self.started
    }
}

impl TerminalSession<PtyTransport> {
    /// Starts the agent's shell - [`SessionConfig::shell`], the user's own
    /// unless a test chose otherwise - in the agent's folder.
    ///
    /// `on_output` sees every chunk and `on_exit` the exit, both on the PTY
    /// reader thread, after the tracker has.
    pub fn spawn_pty(config: &SessionConfig<'_>, sink: EventSink,
                     on_output: impl Fn(&[u8]) + Send + Sync + 'static,
                     on_exit: impl Fn(ExitReport) + Send + Sync + 'static)
                     -> Result<Self> {
        // The tracker needs the terminal to inject through, and the terminal
        // needs its hooks before it starts reading - so the hooks reach the
        // tracker through a slot filled once both exist.
        let tracker_slot: Arc<Mutex<Option<Arc<Tracker>>>> = Arc::default();
        let pull_requests: Arc<Mutex<PullRequestTap>> = Arc::default();
        let command = config.shell.command(&config.agent.folder);
        let terminal = TerminalBuilder::new().on_output({
                                                 let tracker_slot = Arc::clone(&tracker_slot);
                                                 let pull_requests = Arc::clone(&pull_requests);
                                                 move |bytes| {
                                                     let tracker = tracker_slot.lock().clone();
                                                     if let Some(tracker) = tracker {
                                                         tracker.on_terminal_activity();
                                                     }
                                                     pull_requests.lock().feed(bytes);
                                                     on_output(bytes);
                                                 }
                                             })
                                             .on_exit({
                                                 let tracker_slot = Arc::clone(&tracker_slot);
                                                 move |report| {
                                                     let tracker = tracker_slot.lock().clone();
                                                     if let Some(tracker) = tracker {
                                                         tracker.on_process_exit(report.code);
                                                     }
                                                     on_exit(report);
                                                 }
                                             })
                                             .connect(|sink| PtyTransport::spawn(&command, sink))?;
        let tracker = make_tracker(config, terminal.clone(), sink);
        *tracker_slot.lock() = Some(Arc::clone(&tracker));
        Ok(Self { terminal,
                  tracker,
                  pull_requests,
                  started: false })
    }
}

/// `text` and its Return in one write, so a keystroke from the view cannot
/// land between them.
fn send_command<T: Transport>(terminal: &Terminal<T>, text: &str) -> Result<()> {
    let mut line = Vec::with_capacity(text.len() + 1);
    line.extend_from_slice(text.as_bytes());
    line.push(b'\r');
    Ok(terminal.write(&line)?)
}

fn make_tracker<T: Transport>(config: &SessionConfig<'_>, terminal: Terminal<T>,
                              mut sink: EventSink)
                              -> Arc<Tracker> {
    let mut caller_inject = sink.on_inject_registration.take();
    sink.on_inject_registration = Some(Box::new(move |prompt| {
                                           // Best effort: a terminal that is
                                           // gone has no agent left to
                                           // register.
                                           let _ = send_command(&terminal, &prompt);
                                           if let Some(caller_inject) = caller_inject.as_mut() {
                                               caller_inject(prompt);
                                           }
                                       }));
    let tracker =
        Arc::new(Tracker::spawn(TrackerConfig { agent_type: config.agent.agent_type.clone(),
                                                is_hook_based: matches!(config.agent
                                                                              .agent_type
                                                                              .as_str(),
                                                                        "claude" | "codex"),
                                                mcp_enabled: config.settings
                                                                   .mcp_server_enabled,
                                                inline_registration:
                                                    supports_inline_registration(&config.agent
                                                                                        .agent_type),
                                                ..TrackerConfig::default() },
                                tracking_for(&config.agent.agent_type, config.agent.view_mode),
                                sink));
    if config.settings.mcp_server_enabled && !supports_inline_registration(&config.agent.agent_type)
    {
        tracker.set_registration_prompt(registration_prompt(config.agent.id));
    }
    tracker
}

#[cfg(test)]
mod tests;
