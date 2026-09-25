//! Reading Claude Code's tool calls for delegations.
//!
//! Pinned to `@agentclientprotocol/claude-agent-acp`'s wire shape, captured in
//! `tests/fixtures/claude/`. Those fixtures are the contract; a version bump
//! is checked by re-reading the two functions the README names.
//!
//! ## Why the marker rather than the tool name
//!
//! The adapter stamps every tool call with a metadata envelope and marks a
//! delegation in it (`claudeCodeMetaFromToolUse`):
//!
//! ```js
//! return {
//!     toolName: toolUse.name,
//!     ...((toolUse.name === "Agent" || toolUse.name === "Task") && { subagent: true }),
//!     ...
//! };
//! ```
//!
//! That is the adapter's own answer to "is this a delegation" - its
//! `native-subagents` module makes the same two checks to decide what to
//! intercept - so this keys on it rather than on a tool name dug out of
//! `rawInput`. Claude Code's delegation tool has been spelled both `Task` and
//! `Agent`; the marker covers both without this file having to track which.
//!
//! Both checks are needed, not either alone. `subagent: true` is written by
//! `claudeCodeMetaFromToolUse`, and the *completion* update does not call it -
//! that path builds its meta inline as `{ toolName, ...nonExecution }`. A
//! recognizer keyed on the flag alone would see every dispatch and no
//! completion.
//!
//! ## What is deliberately not read
//!
//! `kind` is `"think"` for a delegation, shared with ordinary reasoning calls.
//! `title` is `input.description || "Task"` - prose, with an English fallback.
//! Neither identifies anything, so neither is read here.

use crate::event::SubagentEvent;
use crate::recognize::recognizer::Recognizer;
use crate::recognize::report::ToolCallReport;

/// Claude Code's delegation tool, under both names it has carried.
const DELEGATION_TOOLS: &[&str] = &["Task", "Agent"];

/// Status words that mean the call is still open, so a report carrying one
/// describes a dispatch rather than a completion. An update with no status at
/// all is the adapter's "refine" shape, which also precedes completion.
const OPEN_STATUSES: &[&str] = &["pending", "in_progress"];

const STATUS_COMPLETED: &str = "completed";
const STATUS_FAILED: &str = "failed";

pub struct ClaudeRecognizer;

impl ClaudeRecognizer {
    /// Whether this tool call is a delegation at all.
    fn is_delegation(report: &ToolCallReport<'_>) -> bool {
        if report.meta_bool(&["claudeCode", "subagent"]) == Some(true) {
            return true;
        }

        report.meta_str(&["claudeCode", "toolName"])
              .is_some_and(|name| DELEGATION_TOOLS.contains(&name))
    }
}

impl Recognizer for ClaudeRecognizer {
    fn recognize(&self, report: &ToolCallReport<'_>) -> Option<SubagentEvent> {
        if !Self::is_delegation(report) {
            return None;
        }

        match report.status {
            Some(STATUS_COMPLETED) => SubagentEvent::completed(report.id, "succeeded", None),
            // The reason lives in the call's rendered result on this path;
            // there is no error field of its own to read.
            Some(STATUS_FAILED) => {
                SubagentEvent::completed(report.id, "failed", report.result_text)
            }
            // Open, or an update that changed something other than status.
            // Either way the only thing worth reporting is the dispatch, and
            // `dispatched` answers `None` when the input cannot supply one.
            Some(status) if OPEN_STATUSES.contains(&status) => dispatch(report),
            None => dispatch(report),
            // A status word this adapter has not used before. Reporting
            // nothing leaves the record running and visibly stale, which is
            // the honest outcome - guessing at `completed` would mark a
            // subagent finished on the strength of a word we cannot read.
            Some(_) => None,
        }
    }
}

/// A dispatch built from the delegation's own input fields.
///
/// `subagent_type` and `description` are snake_case: `rawInput` is the model's
/// tool-use input verbatim, not something the adapter renames.
fn dispatch(report: &ToolCallReport<'_>) -> Option<SubagentEvent> {
    SubagentEvent::dispatched(report.id,
                              report.input_str("subagent_type"),
                              report.input_str("description"))
}
