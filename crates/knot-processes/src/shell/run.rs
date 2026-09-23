//! Runs one shell command and reports what it is doing as it does it.
//!
//! The same mechanics as `command`: piped stdio drained on worker threads so a
//! full pipe never wedges the child, and a poll loop that kills on a deadline.
//! What differs is the lifetime. `command::run` blocks until its child is
//! done; a `!` command has to be watched, cancelled and rendered while it
//! runs, so [`spawn`] returns immediately and everything after it happens on a
//! supervisor thread.
//!
//! Nothing here is async. The crate stays runtime-agnostic: the caller polls
//! [`ShellRun::take_dirty`] from wherever it draws.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::decode::take_decodable;
use super::invocation::ShellRequest;
use super::signal::{Signal, signal_group};
use super::state::{ShellRunState, ShellStream};
use super::status::ShellStatus;
use crate::consts::{SHELL_KILL_GRACE, SHELL_POLL_INTERVAL};
use crate::error::ProcessError;

#[cfg(test)]
mod tests;

/// Bytes read from a pipe at a time. Large enough that a chatty command does
/// not wake the decoder per line, small enough that a slow one still reaches
/// the screen promptly.
const READ_CHUNK: usize = 8 * 1024;

/// A running -- or finished -- shell command.
///
/// Cloning gives another handle to the same run, so the supervisor, the
/// canceller and whoever renders all hold one. Dropping every handle does not
/// stop the command: the supervisor owns the child and goes on reaping it.
#[derive(Debug, Clone)]
pub struct ShellRun {
    state:  Arc<Mutex<ShellRunState>>,
    dirty:  Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

impl ShellRun {
    /// Everything captured so far, by value.
    ///
    /// A copy rather than a guard: the caller is usually about to render, and
    /// holding this lock across a frame would put the drain threads behind it.
    pub fn snapshot(&self) -> ShellRunState {
        self.state.lock().clone()
    }

    /// Whether anything observable has changed since this was last asked, and
    /// clears the flag.
    ///
    /// Clearing as it reads: the caller must use the answer on the path that
    /// repaints, never behind an early return that discards it.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    /// Asks the command to stop.
    ///
    /// Returns once the request is recorded, not once the command is gone. The
    /// supervisor signals the process group on its next tick, marks the run
    /// cancelled at that moment, and escalates to `KILL` after
    /// [`SHELL_KILL_GRACE`].
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    /// Whether the run has reached a state it will not leave.
    pub fn is_finished(&self) -> bool {
        self.state.lock().status.is_terminal()
    }

    /// The run's status, by value.
    pub fn status(&self) -> ShellStatus {
        self.state.lock().status.clone()
    }

    fn settle(&self, status: ShellStatus) {
        if self.state.lock().settle(status) {
            self.dirty.store(true, Ordering::Release);
        }
    }

    fn append(&self, stream: ShellStream, chunk: &str) {
        if self.state.lock().append(stream, chunk) {
            self.dirty.store(true, Ordering::Release);
        }
    }
}

/// Starts `request` and hands back a handle to it.
///
/// Never fails: a shell that cannot be launched is a state the run reaches
/// ([`ShellStatus::FailedToStart`]), not an error the caller has to route
/// somewhere else. The command's entry is already on screen by then, and that
/// entry is where the failure belongs.
pub fn spawn(request: ShellRequest) -> ShellRun {
    let run = ShellRun { state:  Arc::new(Mutex::new(ShellRunState::new(request.output_limit))),
                         dirty:  Arc::new(AtomicBool::new(false)),
                         cancel: Arc::new(AtomicBool::new(false)), };

    let supervised = run.clone();
    thread::spawn(move || supervise(request, supervised));

    run
}

/// Owns the child from spawn to reap. Runs on its own thread.
fn supervise(request: ShellRequest, run: ShellRun) {
    let mut child = match start(&request) {
        Ok(child) => child,
        Err(error) => {
            run.settle(ShellStatus::FailedToStart { message: error.to_string(), });
            return;
        }
    };

    let pid = child.id();
    let drains = drain_pipes(&mut child, &run);

    let outcome = wait(&mut child, &run, pid, request.timeout);

    for drain in drains {
        let _ = drain.join();
    }

    run.settle(outcome);
}

fn start(request: &ShellRequest) -> Result<Child, ProcessError> {
    let invocation = request.invocation();

    let mut command = Command::new(&invocation.program);
    command.args(invocation.args)
           .arg(&request.command)
           .current_dir(&request.cwd)
           // Closed, not inherited: a command that reads input gets
           // end-of-file rather than waiting for a terminal that is not there.
           .stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

    // Its own group, so cancelling reaches the whole tree the command starts
    // and not just the shell at its root.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    command.spawn()
           .map_err(|source| ProcessError::ShellLaunch { shell:  request.label(),
                                                         output: source.to_string(), })
}

/// Starts one drain thread per pipe. Each ends when its pipe closes.
fn drain_pipes(child: &mut Child, run: &ShellRun) -> Vec<thread::JoinHandle<()>> {
    let mut drains = Vec::with_capacity(2);

    if let Some(pipe) = child.stdout.take() {
        let run = run.clone();
        drains.push(thread::spawn(move || drain(pipe, run, ShellStream::Stdout)));
    }

    if let Some(pipe) = child.stderr.take() {
        let run = run.clone();
        drains.push(thread::spawn(move || drain(pipe, run, ShellStream::Stderr)));
    }

    drains
}

fn drain(mut pipe: impl Read, run: ShellRun, stream: ShellStream) {
    let mut buffer = vec![0_u8; READ_CHUNK];
    let mut carry: Vec<u8> = Vec::new();

    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                carry.extend_from_slice(&buffer[..read]);
                let text = take_decodable(&mut carry);
                if !text.is_empty() {
                    run.append(stream, &text);
                }
            }
            // The pipe broke, which for this purpose is the same as it
            // closing: there is nothing further to read either way.
            Err(_) => break,
        }
    }

    // Whatever is left cannot be completed -- the pipe is closed.
    if !carry.is_empty() {
        run.append(stream, &String::from_utf8_lossy(&carry));
    }
}

/// Polls for the exit, cancelling and timing out on the way.
///
/// Returns the status the run should settle on, which the caller applies only
/// if the run has not already settled: a cancellation is decided the moment
/// the group is signalled, and the exit that signal causes must not rewrite
/// it.
fn wait(child: &mut Child, run: &ShellRun, pid: u32, timeout: Duration) -> ShellStatus {
    let deadline = Instant::now() + timeout;
    let mut terminated_at: Option<Instant> = None;
    let mut killed = false;

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return match status.code() {
                    Some(code) => ShellStatus::Exited { code },
                    // No code means a signal ended it -- including the one
                    // this loop sent, which `settle` will decline to apply
                    // over the cancellation already recorded.
                    None => ShellStatus::Signalled,
                };
            }
            Ok(None) => {}
            // The child cannot be waited for, so nothing here can learn
            // anything more about it.
            Err(error) => {
                return ShellStatus::FailedToStart { message: error.to_string(), };
            }
        }

        if terminated_at.is_none() {
            if run.cancel.load(Ordering::Acquire) {
                run.settle(ShellStatus::Cancelled);
                signal_group(pid, Signal::Term);
                terminated_at = Some(Instant::now());
            }
            else if Instant::now() >= deadline {
                run.settle(ShellStatus::TimedOut);
                signal_group(pid, Signal::Term);
                terminated_at = Some(Instant::now());
            }
        }
        else if !killed && terminated_at.is_some_and(|at| at.elapsed() >= SHELL_KILL_GRACE) {
            signal_group(pid, Signal::Kill);
            killed = true;
        }

        thread::sleep(SHELL_POLL_INTERVAL);
    }
}
