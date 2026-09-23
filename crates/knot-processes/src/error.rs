use thiserror::Error;

pub type Result<T, E = ProcessError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("{command} timed out")]
    Timeout { command: String },

    #[error("{command} failed (exit {code}): {output}")]
    Command {
        command: String,
        output:  String,
        code:    i32,
    },

    #[error("reading the process table failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("parsing process output failed: {0}")]
    Parse(String),

    /// The PID was reused, or the process left the agent's tree, between the
    /// sample the row was built from and the signal. Nothing was signalled.
    #[error("process {pid} is no longer the process that was listed")]
    IdentityMismatch { pid: u32 },

    /// The signal was rejected -- typically a process owned by another user.
    #[error("signalling process {pid} was refused: {output}")]
    SignalRefused { pid: u32, output: String },
}

#[cfg(test)]
mod tests {
    use super::ProcessError;

    #[test]
    fn timeout_display_names_command() {
        let err = ProcessError::Timeout { command: "ps -Ao pid=".to_owned(), };

        assert!(err.to_string().contains("ps -Ao pid="));
    }

    #[test]
    fn command_display_carries_command_code_and_output() {
        let err = ProcessError::Command { command: "kill -TERM 42".to_owned(),
                                          output:  "no such process".to_owned(),
                                          code:    1, };

        let text = err.to_string();

        assert!(text.contains("kill -TERM 42"));
        assert!(text.contains("exit 1"));
        assert!(text.contains("no such process"));
    }

    #[test]
    fn identity_mismatch_names_the_pid() {
        let err = ProcessError::IdentityMismatch { pid: 4242 };

        assert!(err.to_string().contains("4242"));
    }

    #[test]
    fn signal_refused_carries_pid_and_output() {
        let err = ProcessError::SignalRefused { pid:    17,
                                                output: "Operation not permitted".to_owned(), };

        let text = err.to_string();

        assert!(text.contains("17"));
        assert!(text.contains("Operation not permitted"));
    }
}
