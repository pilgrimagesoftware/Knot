//! Durable application settings and their serialized record types.

mod capabilities;
mod keybindings;
mod records;
mod shared;
mod store;
mod vocabulary;

pub use capabilities::Capabilities;
pub use keybindings::{KeybindingSettings, ShortcutModifiers};
pub use records::{
    ActivationMode, BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, SavedPullRequest,
    SavedWindowBounds, ViewMode, Workspace, WorkspaceUiState,
};
pub use shared::SharedSettings;
pub use store::{Settings, detect_source_base_folder};
pub use vocabulary::{AiProvider, AppearanceMode, AutopilotAction, CostTier, UnknownVariant};
