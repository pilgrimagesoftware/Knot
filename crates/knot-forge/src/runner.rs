//! Running `gh` with a timeout, behind a trait so the rest of the crate can
//! be tested without the binary.

use std::ffi::{OsStr, OsString};
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
///
/// The binary is located through `knot_core::exec_path` rather than left to
/// the process's own `PATH`. Knot is a GUI app: launched from Finder it
/// inherits launchd's `/usr/bin:/bin:/usr/sbin:/sbin`, which names no
/// location `gh` is ever installed in, and the view would report the tool
/// missing on a machine that has it.
#[derive(Debug, Clone)]
pub struct GhRunner {
    timeout:     Duration,
    program:     OsString,
    search_path: OsString,
}

impl Default for GhRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl GhRunner {
    #[must_use]
    pub fn new() -> Self {
        let search_path = knot_core::exec_path::search_path();

        Self { timeout:     DEFAULT_TIMEOUT,
               program:     locate_gh(&search_path),
               search_path: OsString::from(search_path), }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Runs a named program instead of the located `gh`.
    ///
    /// Taken verbatim: an explicit choice of binary is not second-guessed by
    /// a search. Bare names still resolve the way any other spawn does.
    #[must_use]
    pub fn with_program(mut self, program: impl Into<OsString>) -> Self {
        self.program = program.into();
        self
    }

    /// The `PATH` the child is given.
    #[must_use]
    pub fn search_path(&self) -> &OsStr {
        &self.search_path
    }
}

impl ForgeRunner for GhRunner {
    fn run(&self, args: &[&str]) -> Result<String> {
        let label = args.join(" ");

        // No `current_dir`: every command this crate runs names its subject by
        // URL, so the working directory would only decide which repository
        // `gh` guessed at when the URL was already unambiguous.
        // `PATH` as well as the located binary: `gh` shells out itself - to
        // `git` for the repository it is standing in, and to whatever
        // credential helper the user configured - and those lookups run
        // under the environment it is handed.
        let mut child = match Command::new(&self.program).args(args)
                                                         .env("PATH", &self.search_path)
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

/// The `gh` to spawn: the binary located on `search_path`, or the bare name
/// when no directory on it holds one.
///
/// The bare-name fallback is what keeps a genuinely missing tool
/// distinguishable. Spawning it fails with `NotFound`, which becomes
/// [`ForgeError::Missing`], and the view says "not installed" - the one
/// answer that is still true after this change.
fn locate_gh(search_path: &str) -> OsString {
    knot_core::exec_path::resolve_program_on(search_path, GH_PROGRAM).map(OsString::from)
                                                                     .unwrap_or_else(|| {
                                                                         OsString::from(GH_PROGRAM)
                                                                     })
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
