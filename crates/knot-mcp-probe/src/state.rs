//! What state an MCP server is in, as a closed vocabulary.
//!
//! Six values, because the real tooling reports six. Collapsing the
//! deliberate ones into the broken one would tell the user to repair a server
//! they themselves turned off: `claude mcp list` distinguishes `⊘ Disabled
//! for this project` and `⏸ Pending approval` from `✘ Failed to connect`,
//! and so does this.
//!
//! The detail behind a failure is not a variant payload - it lives on
//! [`crate::ServerRow`]. Keeping it out is what lets `Display` and `FromStr`
//! be total and lossless, and the state is the vocabulary; the message is
//! data about one row.

use std::fmt;
use std::str::FromStr;

/// The state of one MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerState {
    /// Reached, and answering.
    Connected,
    /// Reachable, but refusing until the user authenticates.
    NeedsAuthentication,
    /// Configured, and deliberately not connected to until the user approves
    /// it. Not a failure.
    PendingApproval,
    /// Configured, and deliberately turned off. Not a failure.
    Disabled,
    /// Tried and could not be reached. The reason, when there is one, is on
    /// the row.
    Failed,
    /// Reported in terms this crate does not recognize.
    ///
    /// The landing place for output the parser cannot classify. It exists so
    /// that a format change degrades to "there is a server here and I cannot
    /// tell you about it" rather than to silence or to a wrong answer.
    Unknown,
}

/// Every state, so a test can iterate them and fail when one is added
/// without being handled.
pub const ALL_STATES: &[ServerState] = &[ServerState::Connected,
                                         ServerState::NeedsAuthentication,
                                         ServerState::PendingApproval,
                                         ServerState::Disabled,
                                         ServerState::Failed,
                                         ServerState::Unknown];

impl ServerState {
    /// Whether this state is one the user is being asked to do something
    /// about, as opposed to one they chose.
    ///
    /// Drives the collapsed header, which names what needs attention. A
    /// disabled or pending server is deliberate and does not belong in that
    /// count, however much it is "not connected".
    #[must_use]
    pub fn needs_attention(self) -> bool {
        matches!(self, Self::NeedsAuthentication | Self::Failed)
    }

    /// Whether a row in this state offers the delegated action.
    ///
    /// Wider than [`Self::needs_attention`]: re-enabling a disabled server
    /// and approving a pending one are both things the agent's own flow
    /// does, so the action is offered there too. Connected and unknown offer
    /// nothing - there is no remedy to hand over for a server that is
    /// working, and none that is known to help one we cannot classify.
    #[must_use]
    pub fn offers_action(self) -> bool {
        matches!(self,
                 Self::NeedsAuthentication | Self::Failed | Self::PendingApproval | Self::Disabled)
    }

    /// The stable token for this state, used by [`Display`] and [`FromStr`].
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Connected => "connected",
            Self::NeedsAuthentication => "needs-authentication",
            Self::PendingApproval => "pending-approval",
            Self::Disabled => "disabled",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for ServerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.token())
    }
}

/// A token that names no state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownStateToken(pub String);

impl fmt::Display for UnknownStateToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a server state: {}", self.0)
    }
}

impl std::error::Error for UnknownStateToken {}

impl FromStr for ServerState {
    type Err = UnknownStateToken;

    /// Parses a token this module wrote.
    ///
    /// Deliberately strict, and deliberately not the CLI parser: foreign
    /// output that names no state becomes [`ServerState::Unknown`] in
    /// [`crate::parse`], where that is the right answer. Here an unknown
    /// token is a programming error and says so.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ALL_STATES.iter()
                  .copied()
                  .find(|state| state.token() == s)
                  .ok_or_else(|| UnknownStateToken(s.to_owned()))
    }
}

#[cfg(test)]
mod tests;
