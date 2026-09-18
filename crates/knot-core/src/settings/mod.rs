//! Durable application settings and their serialized record types.

mod records;
mod store;

pub use records::{
    BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, ViewMode, Workspace,
};
pub use store::{Settings, detect_source_base_folder};
