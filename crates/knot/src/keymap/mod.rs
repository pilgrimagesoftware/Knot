//! The navigation shortcuts the user can rebind, and the fixed shortcuts
//! they are validated against.
//!
//! Contract: `openspec/specs/keybindings/spec.md`. Design:
//! `openspec/changes/archive/2026-09-25-keybindings/design.md`.
//!
//! [`Resolved`] is the effective set, built from the stored preferences with
//! a default substituted for anything that does not parse or validate.
//! [`apply`] puts a `Resolved` into gpui's keymap, moving only the bindings
//! that changed. The fixed shortcuts live in [`fixed`] so that bootstrap
//! binds from the same list the validator reads.

mod actions;
mod apply;
mod chord;
mod fixed;
mod handlers;
mod resolved;
mod shortcut;
mod validate;

#[cfg(test)]
mod tests;

pub(crate) use actions::*;
pub(crate) use apply::apply;
pub(crate) use apply::apply_and_refresh_menus;
pub(crate) use chord::Chord;
pub(crate) use chord::modifiers_label;
pub(crate) use fixed::fixed_bindings;
pub(crate) use handlers::register_global_handlers;
pub(crate) use resolved::Resolved;
pub(crate) use shortcut::Shortcut;
pub(crate) use validate::validate;
