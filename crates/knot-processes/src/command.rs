//! Runs one short-lived external command with a wall-clock timeout.
//!
//! The same shape as `knot-git`'s runner, minus the working directory: output
//! streams drain on worker threads so a full pipe buffer never wedges the
//! child -- `ps -A` on a busy machine produces far more than a pipe holds --
//! and the calling thread polls for exit and kills on timeout.

use std::ffi::OsString;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::consts::DEFAULT_TIMEOUT;
use crate::error::{ProcessError, Result};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

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
    let stdout_reader = thread::spawn(move || read_to_string(&mut stdout_pipe));
    let stderr_reader = thread::spawn(move || read_to_string(&mut stderr_pipe));

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

    Ok(Output { stdout:  stdout_reader.join().unwrap_or_default(),
                stderr:  stderr_reader.join().unwrap_or_default(),
                code:    status.code().unwrap_or(-1),
                success: status.success(), })
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
