//! Everything one shell run has produced so far.
//!
//! Shared between the drain threads, the supervisor and whoever is rendering,
//! so it is deliberately plain data: two bounded buffers and a status. It
//! knows nothing about processes -- `run` owns those -- and nothing about how
//! any of it is drawn.

use super::buffer::OutputBuffer;
use super::status::ShellStatus;

/// Which of a command's two streams a chunk came from.
///
/// They are captured separately rather than merged: there is no terminal, so
/// merging would invent an interleaving that never existed, and the reader
/// would lose which stream carried a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellStream {
    Stdout,
    Stderr,
}

/// A run's captured output and current status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellRunState {
    pub stdout: OutputBuffer,
    pub stderr: OutputBuffer,
    pub status: ShellStatus,
}

impl ShellRunState {
    /// A running command with nothing captured, each stream bounded at
    /// `limit` bytes.
    pub fn new(limit: usize) -> Self {
        Self { stdout: OutputBuffer::new(limit),
               stderr: OutputBuffer::new(limit),
               status: ShellStatus::Running, }
    }

    /// Appends to one stream, reporting whether a frame would draw this
    /// differently.
    pub fn append(&mut self, stream: ShellStream, chunk: &str) -> bool {
        match stream {
            ShellStream::Stdout => self.stdout.append(chunk),
            ShellStream::Stderr => self.stderr.append(chunk),
        }
    }

    /// Moves the run to `status`, reporting whether that changed anything.
    ///
    /// The first terminal status wins. The supervisor decides a run is
    /// cancelled or timed out at the moment it signals the process group, and
    /// then goes on waiting for the exit that signal causes; that later exit
    /// must not rewrite a cancellation as an ordinary one.
    pub fn settle(&mut self, status: ShellStatus) -> bool {
        if self.status.is_terminal() || self.status == status {
            return false;
        }

        self.status = status;
        true
    }

    /// Whether either stream discarded output at its limit.
    pub fn is_truncated(&self) -> bool {
        self.stdout.is_truncated() || self.stderr.is_truncated()
    }
}

#[cfg(test)]
mod tests {
    use super::{ShellRunState, ShellStatus, ShellStream};

    #[test]
    fn a_new_run_is_running_and_empty() {
        let state = ShellRunState::new(16);

        assert_eq!(state.status, ShellStatus::Running);
        assert!(state.stdout.is_empty());
        assert!(state.stderr.is_empty());
        assert!(!state.is_truncated());
    }

    #[test]
    fn each_stream_is_captured_on_its_own() {
        let mut state = ShellRunState::new(64);

        state.append(ShellStream::Stdout, "out");
        state.append(ShellStream::Stderr, "err");

        assert_eq!(state.stdout.text(), "out");
        assert_eq!(state.stderr.text(), "err");
    }

    #[test]
    fn truncation_of_either_stream_is_reported() {
        let mut state = ShellRunState::new(2);

        state.append(ShellStream::Stderr, "far too long");

        assert!(state.is_truncated());
    }

    #[test]
    fn the_first_terminal_status_wins() {
        let mut state = ShellRunState::new(8);

        assert!(state.settle(ShellStatus::Cancelled));
        assert!(!state.settle(ShellStatus::Exited { code: 143 }),
                "the exit a cancellation caused must not rewrite it");

        assert_eq!(state.status, ShellStatus::Cancelled);
    }

    #[test]
    fn settling_on_the_current_status_is_not_a_change() {
        let mut state = ShellRunState::new(8);

        assert!(!state.settle(ShellStatus::Running));
    }
}
