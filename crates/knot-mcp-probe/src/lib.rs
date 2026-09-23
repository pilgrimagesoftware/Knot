//! Asks an agent's own CLI which MCP servers it has, and in what state.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md`.
//!
//! Distinct from the two crates it sits beside, and the names say which is
//! which: `knot-mcp` *serves* Knot's own MCP server, `knot-mcp-tools`
//! *catalogs* the tools that server exposes, and this one *interrogates*
//! somebody else's. Nothing here knows about Knot's server at all beyond
//! recognizing its endpoint, so that a user who registered it by hand in
//! their agent's configuration does not get two rows for one server.
//!
//! Why a subprocess rather than reading `~/.claude.json`, `.mcp.json` and
//! `~/.codex/config.toml` directly: those files hold the list and never the
//! state, and state is the entire question - "is this server connected" is
//! in no config file. Composing the effective list is also more than it
//! looks, since it spans scope precedence, plugin-contributed servers and
//! per-project approval, all of which the CLI already does correctly.
//!
//! What it costs, and the honesty that buys: the probe is a *separate
//! process* resolving the same configuration, so it is a fresh health check
//! and not a window into the running session. ACP offers nothing better -
//! `session/new`'s `mcpServers` is written once and never read back. Callers
//! must show [`Inventory::taken_at`] rather than implying the rows are live.
//!
//! Runtime-agnostic, like `knot-git` and `knot-forge`: no async runtime here,
//! so wrap calls in `spawn_blocking` at an async boundary.

pub mod consts;
pub mod endpoint;
pub mod error;
pub mod inventory;
pub mod parse;
pub mod probe;
pub mod runner;
pub mod server;
pub mod state;

pub use endpoint::same_endpoint;
pub use error::{ProbeError, Result};
pub use inventory::Inventory;
pub use parse::ListFormat;
pub use probe::{ProbePlan, plan_for, probe};
pub use runner::{CommandRunner, McpRunner, ProbeCommand};
pub use server::{ServerRow, Target};
pub use state::{ALL_STATES, ServerState};
