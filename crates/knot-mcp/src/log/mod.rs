//! The MCP server's diagnostics log.
//!
//! Contract: `openspec/specs/mcp-server/spec.md`.
//!
//! Everything the server reports goes to two places: standard error, which
//! is what it has always printed and what `cargo run` shows, and a rotating
//! file, which is the only record a packaged `Knot.app` leaves behind. The
//! two carry the same events and differ in exactly one respect - the file
//! does not record tool-call argument *values*.
//!
//! Writing is a send, never a syscall: [`Logger`] is a cheap handle over a
//! channel, and one writer task owns the file, the byte counter and the
//! rotation. Nothing on the request path touches the filesystem or takes a
//! lock, and because there is exactly one writer there is no window where a
//! second thread writes into a file that is being rolled.

mod entry;
mod logger;
mod writer;

pub use entry::{Entry, Level, Subject};
pub use logger::Logger;
