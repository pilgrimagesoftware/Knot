//! Reads the operating system's process table and turns it into the live
//! descendant tree of an agent's session root: record parsing, a transitive
//! tree walk, background/foreground classification, and terminating one
//! descendant with `TERM` escalating to `KILL`.
//!
//! Contract: `openspec/specs/agent-processes/spec.md`.
//!
//! Requires `ps` and `kill` on `PATH`. The six `ps` keywords used are
//! POSIX-stable and formatted identically by macOS `ps` and `procps-ng`.
//!
//! Also runs one shell command for the panel's `!` prefix ([`shell`]), which
//! shares the crate's process mechanics but not its lifetime: it streams.
//!
//! Runtime-agnostic: no async runtime anywhere. [`sample`] and [`terminate`]
//! block and want `spawn_blocking` at an async boundary; [`shell::spawn`] does
//! not block and owns the threads it needs.

pub mod command;
pub mod consts;
pub mod error;
pub mod record;
pub mod sample;
pub mod shell;
pub mod table;
pub mod terminate;

pub use error::{ProcessError, Result};
pub use record::{ProcessRecord, parse_table};
pub use sample::sample;
pub use shell::{ShellRequest, ShellRun, ShellRunState, ShellStatus};
pub use table::{Activity, DescendantProcess, ProcessTable};
pub use terminate::{Termination, TerminationTarget, terminate};
