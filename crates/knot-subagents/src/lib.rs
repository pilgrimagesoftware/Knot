//! The agents an agent delegates to: the record Knot keeps of one, the
//! lifecycle it moves through, and the recognizers that turn an agent's own
//! reporting into that record.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md`.
//!
//! A subagent is not a process. Claude Code runs one inside the parent's own
//! process, so `ps` has nothing to report and [`knot_processes`] cannot see
//! it; everything here comes from what the agent itself says, over one of two
//! feeds - the tool-call stream of an ACP session, or the hook events a
//! terminal agent posts. Nothing is inferred from the process table, from
//! terminal output, or from the agent's prose.
//!
//! Runtime-agnostic and UI-free: no async runtime, no GPUI, no subprocess and
//! no I/O. Every recognizer is a pure function from a JSON payload to an
//! optional event, which is what lets the whole surface be tested on captured
//! fixtures.
//!
//! Not to be confused with `knot_core::import::subagents`, which reads other
//! tools' persona *definition* files off disk. That answers what a subagent
//! could be; this answers what one is doing.

pub mod error;
pub mod event;
pub mod kind;
pub mod recognize;
pub mod state;
pub mod subagent;

pub use error::{Result, SubagentError};
pub use event::SubagentEvent;
pub use kind::SubagentKind;
pub use state::{Outcome, SubagentState};
pub use subagent::{Subagent, SubagentId};
