use thiserror::Error;

pub type Result<T, E = SubagentError> = std::result::Result<T, E>;

/// What can go wrong turning a report into a subagent record.
///
/// Deliberately small. Recognition itself does not error: a report this crate
/// does not understand answers `None`, because "that tool call was not a
/// delegation" is the ordinary case, not a failure, and an agent issues far
/// more of those than delegations. These are the cases where a caller gave the
/// crate something it asked for and cannot use.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SubagentError {
    /// A completion named an outcome outside the closed vocabulary. The
    /// recognizers answer `None` rather than raise this; it exists for a
    /// caller parsing an outcome directly, where silently defaulting to
    /// finished would turn a failed subagent into a successful one.
    #[error("unrecognized subagent outcome: {0}")]
    UnknownOutcome(String),

    /// A subagent state string did not round-trip. Same reasoning as above:
    /// the default would be a lie rather than a shrug.
    #[error("unrecognized subagent state: {0}")]
    UnknownState(String),
}

#[cfg(test)]
mod tests {
    use super::SubagentError;

    #[test]
    fn unknown_outcome_names_the_value() {
        let err = SubagentError::UnknownOutcome("cancelled".to_owned());

        assert!(err.to_string().contains("cancelled"));
    }

    #[test]
    fn unknown_state_names_the_value() {
        let err = SubagentError::UnknownState("pending".to_owned());

        assert!(err.to_string().contains("pending"));
    }
}
