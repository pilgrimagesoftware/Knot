//! Runs one shell command for the panel's `!` prefix, streaming what it
//! produces and bounding what it may produce.
//!
//! Contract: `openspec/specs/panel-shell-passthrough/spec.md`.
//!
//! What this owns: choosing the shell, running the command in a given folder
//! with no terminal, capturing both streams under a size limit, ending the
//! command on cancellation or a deadline, and reporting all of it through a
//! dirty flag a renderer can poll.
//!
//! What it does not own: recognising the `!` trigger, deciding which folder to
//! run in, drawing any of this, or deciding whether an agent is told about it.
//! Those are the panel's, in `knot`.
//!
//! Runtime-agnostic like the rest of the crate. [`spawn`] returns immediately
//! and does its work on threads it owns, so there is no blocking entry point
//! here to wrap.

mod buffer;
mod decode;
mod invocation;
mod run;
mod signal;
mod state;
mod status;

pub use buffer::OutputBuffer;
pub use invocation::{ShellInvocation, ShellRequest};
pub use run::{ShellRun, spawn};
pub use state::{ShellRunState, ShellStream};
pub use status::ShellStatus;
