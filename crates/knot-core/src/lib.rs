//! Shared types, error model, constants, localization lookup, and the durable
//! settings store for the Knot Rust port.
//!
//! The `settings` module implements
//! `openspec/specs/settings-persistence/spec.md`; the `import` module
//! implements `openspec/specs/data-import/spec.md`.

rust_i18n::i18n!("locales", fallback = "en");

pub mod agent_type;
pub mod consts;
pub mod error;
pub mod import;
pub mod l10n;
pub mod pull_request_url;
pub mod settings;

pub use error::{Error, Result};
pub use l10n::t;
pub use settings::{
    ActivationMode, AiProvider, AppearanceMode, AutopilotAction, BenchAgent, Capabilities,
    CostTier, Persona, PersonaState, PersonaType, SavedAgent, SavedPullRequest, SavedWindowBounds,
    Settings, UnknownVariant, ViewMode, Workspace, WorkspaceUiState, detect_source_base_folder,
};
