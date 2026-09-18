//! Shared types, error model, constants, localization lookup, and the durable
//! settings store for the Knot Rust port.
//!
//! The `settings` module implements
//! `openspec/specs/settings-persistence/spec.md`.

rust_i18n::i18n!("locales", fallback = "en");

pub mod consts;
pub mod error;
pub mod l10n;
pub mod settings;

pub use error::{Error, Result};
pub use l10n::t;
pub use settings::{
    BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, SavedWindowBounds, Settings,
    ViewMode, Workspace, detect_source_base_folder,
};
