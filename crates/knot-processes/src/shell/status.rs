//! What a shell command's run has come to.
//!
//! A closed vocabulary rather than a string with a default arm: every caller
//! that renders a run has to say what it draws for each of these, and adding a
//! seventh outcome should not compile until it does.

/// The terminal states are distinguished because they read differently to the
/// user and because only [`ShellStatus::Exited`] and [`ShellStatus::Signalled`]
/// -- the two that mean the command ran to its own end -- may be handed to an
/// agent as context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellStatus {
    /// The command is still running.
    Running,
    /// The command exited on its own, with this status.
    Exited { code: i32 },
    /// The command was killed by a signal it did not ask for. The signal
    /// number is not carried: nothing renders it, and `ExitStatus` does not
    /// report one portably.
    Signalled,
    /// The user cancelled the command from its entry.
    Cancelled,
    /// The command was still running at the wall-clock limit.
    TimedOut,
    /// The shell itself could not be launched, so nothing ran.
    FailedToStart { message: String },
}

impl ShellStatus {
    /// Whether the run has reached a state it will not leave.
    pub fn is_terminal(&self) -> bool {
        !matches!(self, Self::Running)
    }

    /// Whether the command ran to its own end -- the only runs whose output may
    /// be attached to a later prompt. A cancelled, timed-out or
    /// failed-to-start run reports what the user already watched happen, and
    /// its output is partial by definition.
    pub fn ran_to_completion(&self) -> bool {
        matches!(self, Self::Exited { .. } | Self::Signalled)
    }

    /// Whether the run should read as a failure. A signal, a timeout and a
    /// cancellation are failures in this sense; only a zero exit is not.
    pub fn is_failure(&self) -> bool {
        !matches!(self, Self::Running | Self::Exited { code: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::ShellStatus;

    #[test]
    fn running_is_the_only_non_terminal_state() {
        assert!(!ShellStatus::Running.is_terminal());

        for status in [ShellStatus::Exited { code: 0 },
                       ShellStatus::Signalled,
                       ShellStatus::Cancelled,
                       ShellStatus::TimedOut,
                       ShellStatus::FailedToStart { message: "no shell".to_owned(), }]
        {
            assert!(status.is_terminal(), "{status:?} should be terminal");
        }
    }

    #[test]
    fn only_a_self_ended_run_may_be_shared() {
        assert!(ShellStatus::Exited { code: 0 }.ran_to_completion());
        assert!(ShellStatus::Exited { code: 3 }.ran_to_completion());
        assert!(ShellStatus::Signalled.ran_to_completion());

        assert!(!ShellStatus::Running.ran_to_completion());
        assert!(!ShellStatus::Cancelled.ran_to_completion());
        assert!(!ShellStatus::TimedOut.ran_to_completion());
        assert!(!ShellStatus::FailedToStart { message: "no shell".to_owned(), }.ran_to_completion());
    }

    #[test]
    fn only_a_zero_exit_is_not_a_failure() {
        assert!(!ShellStatus::Exited { code: 0 }.is_failure());
        assert!(!ShellStatus::Running.is_failure());

        assert!(ShellStatus::Exited { code: 1 }.is_failure());
        assert!(ShellStatus::Cancelled.is_failure());
        assert!(ShellStatus::TimedOut.is_failure());
    }
}
