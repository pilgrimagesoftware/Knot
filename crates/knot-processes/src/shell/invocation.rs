//! What to run, where, and under which shell.
//!
//! Owns the request a caller builds and the choice of shell it resolves to. It
//! does not run anything -- `run` does -- so the shell-selection rule can be
//! tested without a process.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use crate::consts::{
    SHELL_FALLBACK, SHELL_FALLBACK_ARGS, SHELL_LOGIN_ARGS, SHELL_OUTPUT_LIMIT, SHELL_TIMEOUT,
};

/// The program and arguments one command will be run through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellInvocation {
    pub program: OsString,
    pub args:    &'static [&'static str],
}

/// One shell command to run.
#[derive(Debug, Clone)]
pub struct ShellRequest {
    /// The command line, exactly as the user typed it after the `!`.
    pub command:      String,
    /// The agent's folder. A command observes no state from any command run
    /// before it, so this is the only thing placing it anywhere in particular.
    pub cwd:          PathBuf,
    pub timeout:      Duration,
    /// Bytes kept per stream.
    pub output_limit: usize,
    /// Overrides the user's shell. Set only by tests, which need a shell that
    /// cannot launch and one that does not depend on the runner's `SHELL`.
    pub shell:        Option<OsString>,
}

impl ShellRequest {
    /// A request with the standard bounds.
    pub fn new(command: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        Self { command:      command.into(),
               cwd:          cwd.into(),
               timeout:      SHELL_TIMEOUT,
               output_limit: SHELL_OUTPUT_LIMIT,
               shell:        None, }
    }

    /// The shell this request runs through.
    ///
    /// The user's `SHELL` as a login shell, because that is what makes
    /// `!npm test` find `npm`: the application is launched from Finder with an
    /// environment that has none of what the user's profile puts on `PATH`.
    /// `/bin/sh` without `-l` when `SHELL` says nothing useful.
    pub fn invocation(&self) -> ShellInvocation {
        if let Some(shell) = &self.shell {
            return ShellInvocation { program: shell.clone(),
                                     args:    SHELL_LOGIN_ARGS, };
        }

        match std::env::var_os("SHELL") {
            Some(shell) if !shell.is_empty() => ShellInvocation { program: shell,
                                                                  args:    SHELL_LOGIN_ARGS, },
            _ => ShellInvocation { program: SHELL_FALLBACK.into(),
                                   args:    SHELL_FALLBACK_ARGS, },
        }
    }

    /// How the run is named in an error. Not what is executed -- that is the
    /// invocation plus the command as one argument.
    pub fn label(&self) -> String {
        let invocation = self.invocation();
        let mut label = invocation.program.to_string_lossy().into_owned();

        for arg in invocation.args {
            label.push(' ');
            label.push_str(arg);
        }

        label
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;

    use super::ShellRequest;
    use crate::consts::{
        SHELL_FALLBACK, SHELL_FALLBACK_ARGS, SHELL_LOGIN_ARGS, SHELL_OUTPUT_LIMIT, SHELL_TIMEOUT,
    };

    #[test]
    fn a_new_request_carries_the_standard_bounds() {
        let request = ShellRequest::new("ls -la", Path::new("/tmp"));

        assert_eq!(request.timeout, SHELL_TIMEOUT);
        assert_eq!(request.output_limit, SHELL_OUTPUT_LIMIT);
        assert_eq!(request.command, "ls -la");
    }

    #[test]
    fn an_override_runs_as_a_login_shell() {
        let mut request = ShellRequest::new("echo hi", Path::new("/tmp"));
        request.shell = Some(OsString::from("/usr/bin/knot-no-such-shell"));

        let invocation = request.invocation();

        assert_eq!(invocation.program,
                   OsString::from("/usr/bin/knot-no-such-shell"));
        assert_eq!(invocation.args, SHELL_LOGIN_ARGS);
    }

    #[test]
    fn the_fallback_shell_drops_the_login_flag() {
        // Asserted through the override-free path would mean mutating the
        // process's environment, which every other test in this binary shares.
        // The rule itself is the constant pairing, so assert that.
        assert_eq!(SHELL_FALLBACK, "/bin/sh");
        assert_eq!(SHELL_FALLBACK_ARGS, &["-c"]);
        assert_eq!(SHELL_LOGIN_ARGS, &["-lc"]);
    }

    #[test]
    fn the_label_names_the_shell_and_its_flags() {
        let mut request = ShellRequest::new("echo hi", Path::new("/tmp"));
        request.shell = Some(OsString::from("/bin/zsh"));

        assert_eq!(request.label(), "/bin/zsh -lc");
    }
}
