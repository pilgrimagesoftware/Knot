//! The processes section: what the selected agent has left running.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "The agent's pane
//! presents a processes section".
//!
//! Split by concern rather than by size. `section` owns the disclosure, the
//! header and the body; `process_row` owns one operating-system process;
//! `chrome` owns what any row kind shares; `empty` and `text` own the copy.
//! The split came before the section gained a second kind of row, so that
//! addition lands as a sibling rather than as an append to a file already
//! over half the line limit.
//!
//! It reads only what the sampler last published; nothing here enumerates
//! processes, which `agent_processes`'s own test enforces over these sources.

pub(super) mod chrome;
pub(super) mod empty;
pub(super) mod process_row;
pub(super) mod section;
pub(super) mod text;

#[cfg(test)]
mod tests;
