//! Reads pull request state from GitHub through the `gh` CLI.
//!
//! Contract:
//! `openspec/changes/pull-request-tracking/specs/pull-request-tracking/spec.
//! md`.
//!
//! Separate from `knot-git` on purpose. `knot-git` is about the local
//! repository: no network, no credentials, and a `git` binary that is not
//! optional. A forge client has all three of those the other way around, and
//! folding it in would make `knot-git`'s tests depend on `gh` being installed
//! and authenticated.
//!
//! `gh` rather than an HTTP client because it already holds the user's
//! credentials, already knows their Enterprise hosts, and refreshes its own
//! tokens. The alternative is Knot storing a GitHub token, which is a larger
//! change than reading a pull request's title is worth.
//!
//! The binary is optional. [`probe`] reports its absence as
//! [`ForgeAvailability::Missing`] rather than an error, because a machine
//! without `gh` is one where recorded pull requests still list and still open
//! in a browser - only their state is absent.
//!
//! Runtime-agnostic, for the same reason as `knot-git`: no async runtime here,
//! so wrap calls in `spawn_blocking` at an async boundary.

pub mod availability;
pub mod consts;
pub mod error;
pub mod pull_request;
pub mod runner;

pub use availability::{ForgeAvailability, probe, probe_with};
pub use error::{ForgeError, Result};
pub use pull_request::{
    CheckRollup, PullRequestState, PullRequestStatus, pull_request_state, pull_request_state_with,
};
pub use runner::{ForgeRunner, GhRunner};
