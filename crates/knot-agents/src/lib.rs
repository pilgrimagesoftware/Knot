//! Runtime agent lifecycle: creation, removal, restart/resume, edit, bench
//! deployment, and ordering, layered over `knot_core`'s durable settings
//! store.
//!
//! Contract: `openspec/specs/agent-lifecycle/spec.md`.

mod agent;
mod convert;
mod duplicate_name;
mod error;
mod registry;
mod store;

pub use agent::{Agent, AgentState};
pub use convert::{from_saved, to_saved};
pub use duplicate_name::duplicate_name;
pub use error::{AgentError, Result};
pub use registry::{RegistryEntry, RegistryQuery, RegistryStatus, RegistryView, visible_to};
pub use store::{AdoptedCounts, AgentStore, CreateOptions, EditRequest, RemovedAgent};
