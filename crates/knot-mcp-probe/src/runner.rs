//! Running an agent's own MCP list command, behind a trait so the rest of
//! the crate is testable without any agent CLI installed.
//!
//! The command runs **in the agent's own working directory and environment**,
//! not Knot's. Project-scoped configuration resolves relative to the working
//! directory, and variables like `CLAUDE_CONFIG_DIR` decide which user
//! configuration is in play, so a probe run from Knot's own cwd would
//! silently answer a different question than the one the user is looking at.
//!
//! Stdin is closed. A CLI that would prompt then fails immediately instead of
//! sitting there until [`PROBE_TIMEOUT`] - the difference between a clear
//! failure and a section that says "checking" for thirty seconds.

use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::consts::{POLL_INTERVAL, PROBE_TIMEOUT};
use crate::error::{ProbeError, Result};

/// One invocation: what to run, where, and with what environment.
///
/// A struct rather than five arguments, and the reason is not only the
/// argument-count rule - the working directory and the environment are not
/// incidental parameters, they are what makes the answer the *agent's*
/// answer, and grouping them keeps them from being forgotten one at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCommand {
    pub program: String,
    pub args:    Vec<String>,
    /// The agent's working directory: what project-scoped configuration
    /// resolves against.
    pub cwd:     PathBuf,
    /// The environment Knot launched the agent with.
    pub env:     Vec<(String, String)>,
}

impl ProbeCommand {
    #[must_use]
    pub fn new(program: impl Into<String>, args: Vec<String>, cwd: impl Into<PathBuf>) -> Self {
        Self { program: program.into(),
               args,
               cwd: cwd.into(),
               env: Vec::new() }
    }

    #[must_use]
    pub fn with_env(mut self, env: Vec<(String, String)>) -> Self {
        self.env = env;
        self
    }

    /// The command as one line, for a message that names what failed.
    #[must_use]
    pub fn label(&self) -> String {
        if self.args.is_empty() {
            return self.program.clone();
        }

        format!("{} {}", self.program, self.args.join(" "))
    }
}

/// Runs an agent's MCP list command and hands back its stdout.
pub trait McpRunner {
    fn run(&self, command: &ProbeCommand) -> Result<String>;
}

/// Runs the real CLI.
///
/// Output streams drain on worker threads so a full pipe buffer never wedges
/// the child; the main thread polls for exit and kills on timeout. The same
/// shape as `knot_forge::GhRunner` and `knot_git::Runner`, for the same
/// reason.
#[derive(Debug, Clone)]
pub struct CommandRunner {
    timeout: Duration,
}

impl Default for CommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunner {
    #[must_use]
    pub fn new() -> Self {
        Self { timeout: PROBE_TIMEOUT, }
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl McpRunner for CommandRunner {
    fn run(&self, command: &ProbeCommand) -> Result<String> {
        let program = locate(&command.program, &command.env);

        let mut child = match Command::new(&program).args(&command.args)
                                                    .current_dir(&command.cwd)
                                                    .envs(command.env.iter().map(|(k, v)| (k, v)))
                                                    .stdin(Stdio::null())
                                                    .stdout(Stdio::piped())
                                                    .stderr(Stdio::piped())
                                                    .spawn()
        {
            Ok(child) => child,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(ProbeError::Missing { program: command.program.clone(), });
            }
            Err(err) => {
                return Err(ProbeError::Io { program: command.program.clone(),
                                            message: err.to_string(), });
            }
        };

        let mut stdout_pipe = child.stdout.take().expect("stdout piped");
        let mut stderr_pipe = child.stderr.take().expect("stderr piped");
        let stdout_reader = thread::spawn(move || read_to_string(&mut stdout_pipe));
        let stderr_reader = thread::spawn(move || read_to_string(&mut stderr_pipe));

        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {}
                Err(err) => {
                    return Err(ProbeError::Io { program: command.program.clone(),
                                                message: err.to_string(), });
                }
            }

            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                drop(stdout_reader);
                drop(stderr_reader);

                return Err(ProbeError::TimedOut { program: command.label(),
                                                  seconds: self.timeout.as_secs(), });
            }

            thread::sleep(POLL_INTERVAL);
        };

        let stdout = stdout_reader.join().unwrap_or_default();
        let stderr = stderr_reader.join().unwrap_or_default();

        if status.success() {
            return Ok(stdout);
        }

        // A CLI that health-checks as it lists can exit non-zero *and* have
        // listed: `claude mcp list` does exactly that when a server fails to
        // connect. Stdout that still holds rows is the answer; the exit code
        // alone would throw away the very state the section exists to show.
        if !stdout.trim().is_empty() {
            return Ok(stdout);
        }

        let output = if stderr.trim().is_empty() {
            stdout
        }
        else {
            stderr
        };

        Err(ProbeError::Command { program: command.label(),
                                  output:  output.trim().to_owned(),
                                  code:    status.code().unwrap_or(-1), })
    }
}

/// The binary to spawn: resolved on the agent's own `PATH` when it has one,
/// otherwise on Knot's.
///
/// Knot is a GUI app: launched from Finder it inherits launchd's
/// `/usr/bin:/bin:/usr/sbin:/sbin`, which names no directory any agent CLI is
/// installed in. `knot_core::exec_path` is the existing answer to that, and
/// the agent's own `PATH` takes precedence because the agent was launched
/// with it and is the thing whose configuration we are asking about.
///
/// The bare-name fallback is what keeps a genuinely missing tool
/// distinguishable: spawning it fails with `NotFound`, which becomes
/// [`ProbeError::Missing`], and the section says "not installed".
fn locate(program: &str, env: &[(String, String)]) -> OsString {
    let path = env.iter()
                  .find(|(key, _)| key == "PATH")
                  .map(|(_, value)| value.clone())
                  .unwrap_or_else(knot_core::exec_path::search_path);

    knot_core::exec_path::resolve_program_on(&path, program).map(OsString::from)
                                                            .unwrap_or_else(|| {
                                                                OsString::from(program)
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
