//! Running `gh` with a timeout, behind a trait so the rest of the crate can
//! be tested without the binary.

use std::ffi::OsString;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::consts::{DEFAULT_TIMEOUT, GH_PROGRAM, POLL_INTERVAL};
use crate::error::{ForgeError, Result};

/// Runs a forge command and hands back its stdout.
///
/// A trait rather than a concrete type because every decision this crate
/// makes (available, authenticated, open, merged, closed) is a decision about
/// what came back from a subprocess, and a test that has to install and
/// authenticate `gh` to check them is a test nobody can run.
pub trait ForgeRunner {
    /// Runs `gh <args>`, returning stdout trimmed of surrounding whitespace.
    fn run(&self, args: &[&str]) -> Result<String>;
}

/// Runs the real `gh` binary.
///
/// Output streams drain on worker threads so a full pipe buffer never wedges
/// the child; the main thread polls for exit and kills on timeout. The same
/// shape as `knot_git::Runner`, for the same reason.
#[derive(Debug, Clone)]
pub struct GhRunner {
    timeout: Duration,
    program: OsString,
}

impl Default for GhRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl GhRunner {
    #[must_use]
    pub fn new() -> Self {
        Self { timeout: DEFAULT_TIMEOUT,
               program: OsString::from(GH_PROGRAM), }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn with_program(mut self, program: impl Into<OsString>) -> Self {
        self.program = program.into();
        self
    }
}

impl ForgeRunner for GhRunner {
    fn run(&self, args: &[&str]) -> Result<String> {
        let label = args.join(" ");

        // No `current_dir`: every command this crate runs names its subject by
        // URL, so the working directory would only decide which repository
        // `gh` guessed at when the URL was already unambiguous.
        let mut child = match Command::new(&self.program).args(args)
                                                         .stdin(Stdio::null())
                                                         .stdout(Stdio::piped())
                                                         .stderr(Stdio::piped())
                                                         .spawn()
        {
            Ok(child) => child,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(ForgeError::Missing);
            }
            Err(err) => return Err(err.into()),
        };

        let mut stdout_pipe = child.stdout.take().expect("stdout piped");
        let mut stderr_pipe = child.stderr.take().expect("stderr piped");
        let stdout_reader = thread::spawn(move || read_to_string(&mut stdout_pipe));
        let stderr_reader = thread::spawn(move || read_to_string(&mut stderr_pipe));

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                drop(stdout_reader);
                drop(stderr_reader);
                return Err(ForgeError::Timeout { command: label });
            }

            thread::sleep(POLL_INTERVAL);
        };

        let stdout = stdout_reader.join().unwrap_or_default();
        let stderr = stderr_reader.join().unwrap_or_default();

        if status.success() {
            return Ok(stdout.trim().to_owned());
        }

        let output = if stderr.trim().is_empty() {
            stdout
        }
        else {
            stderr
        };

        Err(ForgeError::Command { command: label,
                                  output:  output.trim().to_owned(),
                                  code:    status.code().unwrap_or(-1), })
    }
}

fn read_to_string(pipe: &mut impl Read) -> String {
    let mut buf = String::new();
    let _ = pipe.read_to_string(&mut buf);
    buf
}

#[cfg(test)]
pub(crate) mod stub;

#[cfg(test)]
mod tests;
