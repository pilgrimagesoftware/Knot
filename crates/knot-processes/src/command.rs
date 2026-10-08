//! Runs one short-lived external command with a wall-clock timeout.
//!
//! The same shape as `knot-git`'s runner, minus the working directory: output
//! streams drain on worker threads so a full pipe buffer never wedges the
//! child -- `ps -A` on a busy machine produces far more than a pipe holds --
//! and the calling thread polls for exit and kills on timeout.
//!
//! The timeout bounds the whole call, output included. A child can exit and
//! leave a background process of its own holding the output pipes open, and
//! the streams only end when the last holder closes them; waiting on the
//! drain threads without a bound made the call last as long as that
//! process, whatever the timeout said.

use std::ffi::OsString;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::consts::{DEFAULT_TIMEOUT, OUTPUT_DRAIN_FLOOR, POLL_INTERVAL};
use crate::error::{ProcessError, Result};

/// What a finished command produced, without judging its exit status.
#[derive(Debug, Clone)]
pub struct Output {
    pub stdout:  String,
    pub stderr:  String,
    pub code:    i32,
    pub success: bool,
}

impl Output {
    /// Whichever stream carries the failure text, trimmed. `stderr` unless it
    /// is empty, in which case the command wrote its complaint to `stdout`.
    pub fn failure_text(&self) -> String {
        if self.stderr.trim().is_empty() {
            self.stdout.trim().to_owned()
        }
        else {
            self.stderr.trim().to_owned()
        }
    }
}

/// Runs `program args...` and returns its output whatever the exit status.
///
/// Use this when a non-zero exit is information rather than an error -- `kill`
/// reporting that a process has already gone, for instance.
pub fn run(program: impl Into<OsString>, args: &[&str]) -> Result<Output> {
    run_with_timeout(program, args, DEFAULT_TIMEOUT)
}

/// Runs `program args...` and fails on a non-zero exit, returning stdout.
pub fn run_checked(program: impl Into<OsString>, args: &[&str]) -> Result<String> {
    let program = program.into();
    let label = display_command(&program, args);
    let output = run_with_timeout(program, args, DEFAULT_TIMEOUT)?;

    if output.success {
        return Ok(output.stdout);
    }

    Err(ProcessError::Command { command: label,
                                output:  output.failure_text(),
                                code:    output.code, })
}

pub fn run_with_timeout(program: impl Into<OsString>, args: &[&str], timeout: Duration)
                        -> Result<Output> {
    let program = program.into();
    let label = display_command(&program, args);

    let mut child = Command::new(&program).args(args)
                                          .stdin(Stdio::null())
                                          .stdout(Stdio::piped())
                                          .stderr(Stdio::piped())
                                          .spawn()?;

    let mut stdout_pipe = child.stdout.take().expect("stdout piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr piped");
    let stdout_reader = drain(move || read_to_string(&mut stdout_pipe));
    let stderr_reader = drain(move || read_to_string(&mut stderr_pipe));

    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            drop(stdout_reader);
            drop(stderr_reader);
            return Err(ProcessError::Timeout { command: label });
        }

        thread::sleep(POLL_INTERVAL);
    };

    let (Some(stdout), Some(stderr)) =
        (collect(&stdout_reader, deadline), collect(&stderr_reader, deadline))
    else {
        return Err(ProcessError::Timeout { command: label });
    };

    Ok(Output { stdout,
                stderr,
                code: status.code().unwrap_or(-1),
                success: status.success() })
}

/// Reads a stream to its end on a thread of its own, handing the text back
/// through the returned channel. The thread is detached: one still blocked
/// when [`collect`] gives up ends when the stream finally closes.
fn drain(read: impl FnOnce() -> String + Send + 'static) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(read());
    });
    rx
}

/// A drained stream's text, or `None` if it has not ended by `deadline` -
/// with at least [`OUTPUT_DRAIN_FLOOR`] allowed, so a child that exits just
/// before the deadline is not reported as timed out for want of a moment to
/// flush.
fn collect(reader: &mpsc::Receiver<String>, deadline: Instant) -> Option<String> {
    let wait = deadline.saturating_duration_since(Instant::now())
                       .max(OUTPUT_DRAIN_FLOOR);
    reader.recv_timeout(wait).ok()
}

fn read_to_string(pipe: &mut impl Read) -> String {
    let mut buf = String::new();
    let _ = pipe.read_to_string(&mut buf);
    buf
}

fn display_command(program: &OsString, args: &[&str]) -> String {
    let mut label = program.to_string_lossy().into_owned();

    for arg in args {
        label.push(' ');
        label.push_str(arg);
    }

    label
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{run, run_checked, run_with_timeout};
    use crate::error::ProcessError;

    #[test]
    fn checked_run_returns_stdout() {
        let out = run_checked("echo", &["hello"]).unwrap();

        assert_eq!(out.trim(), "hello");
    }

    #[test]
    fn checked_run_reports_a_non_zero_exit() {
        let err = run_checked("false", &[]).unwrap_err();

        match err {
            ProcessError::Command { code, .. } => assert_ne!(code, 0),
            other => panic!("expected Command, got {other:?}"),
        }
    }

    #[test]
    fn unchecked_run_reports_a_non_zero_exit_as_output() {
        let out = run("false", &[]).unwrap();

        assert!(!out.success);
    }

    #[test]
    fn timeout_kills_the_child_and_names_the_command() {
        let started = Instant::now();
        let err = run_with_timeout("sleep", &["5"], Duration::from_millis(50)).unwrap_err();

        assert!(started.elapsed() < Duration::from_secs(2),
                "did not abort early");
        match err {
            ProcessError::Timeout { command } => assert_eq!(command, "sleep 5"),
            other => panic!("expected Timeout, got {other:?}"),
        }
    }

    /// The child exits at once, but a background process it started keeps
    /// stdout open for far longer than the timeout. The call must still end
    /// at the timeout rather than when that process does.
    #[test]
    fn a_background_process_holding_the_output_does_not_outlast_the_timeout() {
        let started = Instant::now();
        let err = run_with_timeout("sh",
                                   &["-c", "sleep 30 & echo started"],
                                   Duration::from_millis(200)).unwrap_err();

        assert!(started.elapsed() < Duration::from_secs(5),
                "waited {:?} on a pipe the child's background process held",
                started.elapsed());
        assert!(matches!(err, ProcessError::Timeout { .. }), "got {err:?}");
    }

    #[test]
    fn a_missing_program_is_an_io_error() {
        let err = run_checked("knot-no-such-program", &[]).unwrap_err();

        assert!(matches!(err, ProcessError::Io(_)));
    }

    #[test]
    fn failure_text_prefers_stderr_and_falls_back_to_stdout() {
        let out = run("sh", &["-c", "echo out; echo err >&2; exit 3"]).unwrap();

        assert_eq!(out.failure_text(), "err");
        assert_eq!(out.code, 3);

        let out = run("sh", &["-c", "echo only-out; exit 4"]).unwrap();

        assert_eq!(out.failure_text(), "only-out");
    }
}
