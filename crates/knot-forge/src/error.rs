use thiserror::Error;

pub type Result<T, E = ForgeError> = std::result::Result<T, E>;

/// What can go wrong reading from the forge.
///
/// [`ForgeError::Missing`] is separate from [`ForgeError::Io`] because it is
/// the one failure that is not a failure: a machine without `gh` is a
/// supported configuration, and the view says so once rather than per row.
#[derive(Debug, Error)]
pub enum ForgeError {
    #[error("gh is not installed")]
    Missing,

    #[error("gh is not authenticated")]
    Unauthenticated,

    #[error("gh {command} timed out")]
    Timeout { command: String },

    #[error("gh {command} failed (exit {code}): {output}")]
    Command {
        command: String,
        output:  String,
        code:    i32,
    },

    #[error("running gh failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("parsing gh output failed: {0}")]
    Parse(String),
}

impl ForgeError {
    /// Whether this is the kind of failure that will still be true on the next
    /// refresh. A missing or unauthenticated binary is worth reporting once
    /// for the view; a timeout or a bad response is worth retrying.
    #[must_use]
    pub fn is_persistent(&self) -> bool {
        matches!(self, Self::Missing | Self::Unauthenticated)
    }
}

#[cfg(test)]
mod tests {
    use super::ForgeError;

    #[test]
    fn command_display_carries_command_code_and_output() {
        let err = ForgeError::Command { command: "pr view https://example".to_owned(),
                                        output:  "no pull requests found".to_owned(),
                                        code:    1, };
        let text = err.to_string();

        assert!(text.contains("pr view https://example"));
        assert!(text.contains("exit 1"));
        assert!(text.contains("no pull requests found"));
    }

    #[test]
    fn only_the_two_standing_conditions_are_persistent() {
        assert!(ForgeError::Missing.is_persistent());
        assert!(ForgeError::Unauthenticated.is_persistent());
        assert!(!ForgeError::Timeout { command: "pr view".to_owned(), }.is_persistent());
        assert!(!ForgeError::Parse("bad json".to_owned()).is_persistent());
    }
}
