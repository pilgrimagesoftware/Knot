//! The recognizer trait, and which one an agent type gets.

use crate::event::SubagentEvent;
use crate::recognize::claude::ClaudeRecognizer;
use crate::recognize::report::ToolCallReport;

/// Reads one agent type's tool calls for delegations.
pub trait Recognizer: Send + Sync {
    /// The event this tool call reports, or `None` if it reports none.
    ///
    /// `None` is the ordinary answer: most tool calls are not delegations.
    fn recognize(&self, report: &ToolCallReport<'_>) -> Option<SubagentEvent>;
}

/// The recognizer for `agent_type`, or `None` for a type whose adapter has
/// not been read.
///
/// A type with no recognizer is not a gap to paper over: it is what
/// `SubagentReporting::None` on the roster describes, and the processes
/// section renders it as "cannot tell" rather than "dispatched none". Adding
/// a type here is one arm plus its fixtures, and the roster column flips in
/// the same commit.
#[must_use]
pub fn for_agent_type(agent_type: &str) -> Option<&'static dyn Recognizer> {
    match agent_type {
        "claude" => Some(&ClaudeRecognizer),
        _ => None,
    }
}
