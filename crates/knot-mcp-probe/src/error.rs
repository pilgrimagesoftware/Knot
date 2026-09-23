use thiserror::Error;

pub type Result<T, E = ProbeError> = std::result::Result<T, E>;

/// What can go wrong asking an agent's own CLI which MCP servers it has.
///
/// Every variant is something the section reports in words. There is no
/// variant meaning "no servers": that is an empty [`crate::Inventory`], and
/// conflating the two is the mistake this enum exists to make impossible.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProbeError {
    #[error("{program} is not installed")]
    Missing { program: String },

    #[error("{program} timed out after {seconds}s")]
    TimedOut { program: String, seconds: u64 },

    #[error("{program} failed (exit {code}): {output}")]
    Command {
        program: String,
        output:  String,
        code:    i32,
    },

    /// The command ran and said something, and none of it was a server.
    ///
    /// Distinct from an empty inventory: this is output in a shape the parser
    /// does not know, which means the format moved rather than that the agent
    /// has nothing configured. It carries the first line so the section can
    /// show what it could not read.
    #[error("could not read {program}'s output: {first_line}")]
    Unrecognized {
        program:    String,
        first_line: String,
    },

    #[error("running {program} failed: {message}")]
    Io { program: String, message: String },
}

impl ProbeError {
    /// The program the failure is about, so a message can name it.
    #[must_use]
    pub fn program(&self) -> &str {
        match self {
            Self::Missing { program }
            | Self::TimedOut { program, .. }
            | Self::Command { program, .. }
            | Self::Unrecognized { program, .. }
            | Self::Io { program, .. } => program,
        }
    }

    /// Whether this failure will still be true on the next probe.
    ///
    /// A missing binary is worth saying once rather than retrying on every
    /// refresh; a timeout or a bad exit is worth another look.
    #[must_use]
    pub fn is_persistent(&self) -> bool {
        matches!(self, Self::Missing { .. })
    }
}

#[cfg(test)]
mod tests;
