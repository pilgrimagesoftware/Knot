//! Where a window reaches the process's settings surface.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! [`knot_core::SharedSettings`] is the surface; this is how `knot` gets at
//! it. It is a GPUI global for the same reason [`WindowRegistry`] and
//! `QuitGuard` are: every window needs it, none of them owns it, and
//! threading it through each constructor is what produced the per-window
//! copies of #238 in the first place.
//!
//! The handle itself lives in `knot-core` rather than here because two
//! writers do - the persona and workspace imports - and `knot-core` cannot see
//! `gpui`.
//!
//! [`read`] returns a value fixed at the moment of the call. Take one per
//! frame or per function and let it drop; a field that holds one is the
//! window-lifetime snapshot this change exists to remove.
//!
//! [`WindowRegistry`]: crate::window_registry::WindowRegistry

use std::sync::Arc;

use gpui_kit::App;
use knot_core::{Settings, SharedSettings};

/// The process's settings surface, as a GPUI global.
// UNWIRED(#401): stage 1 installs the surface; the readers and writers move
// over in stage 2, one owner struct at a time. Until then nothing outside the
// tests calls these, so `dead_code` is correct and temporary - deleting the
// allow is part of the last conversion, not a follow-up.
#[allow(dead_code)]
pub(crate) struct SettingsGlobal(SharedSettings);

impl gpui_kit::Global for SettingsGlobal {}

/// Installs `settings` as the shared surface, if one is not installed already.
///
/// Idempotent so a test that opens a window can call it without caring
/// whether the bootstrap already ran. The first caller wins: a second one
/// replacing the surface would hand the windows already holding the first a
/// value nothing else writes to, which is the bug rather than a fix for it.
pub(crate) fn install(settings: Settings, cx: &mut App) {
    if !cx.has_global::<SettingsGlobal>() {
        cx.set_global(SettingsGlobal(SharedSettings::new(settings)));
    }
}

/// A handle to the shared surface.
///
/// # Panics
///
/// If no surface is installed. That is a wiring mistake rather than a runtime
/// condition - [`install`] runs before the first window opens - so it fails
/// loudly instead of quietly substituting a value that writes somewhere the
/// caller did not expect.
// UNWIRED(#401): stage 1 installs the surface; the readers and writers move
// over in stage 2, one owner struct at a time. Until then nothing outside the
// tests calls these, so `dead_code` is correct and temporary - deleting the
// allow is part of the last conversion, not a follow-up.
#[allow(dead_code)]
pub(crate) fn handle(cx: &App) -> SharedSettings {
    cx.global::<SettingsGlobal>().0.clone()
}

/// The settings as they stand now.
///
/// # Panics
///
/// See [`handle`].
// UNWIRED(#401): stage 1 installs the surface; the readers and writers move
// over in stage 2, one owner struct at a time. Until then nothing outside the
// tests calls these, so `dead_code` is correct and temporary - deleting the
// allow is part of the last conversion, not a follow-up.
#[allow(dead_code)]
pub(crate) fn read(cx: &App) -> Arc<Settings> {
    cx.global::<SettingsGlobal>().0.read()
}

/// Applies `change` to the shared surface and returns the installed value.
///
/// Takes `&App` rather than `&mut App`: the surface is interior-mutable, so a
/// writer does not need exclusive access to the app. That is what lets a
/// caller write while holding other borrows, and it keeps writes off
/// `global_mut`, which would contend with every other global.
///
/// `change` may run more than once - see [`SharedSettings::write`].
///
/// # Panics
///
/// See [`handle`].
// UNWIRED(#401): stage 1 installs the surface; the readers and writers move
// over in stage 2, one owner struct at a time. Until then nothing outside the
// tests calls these, so `dead_code` is correct and temporary - deleting the
// allow is part of the last conversion, not a follow-up.
#[allow(dead_code)]
pub(crate) fn write<F>(cx: &App, change: F) -> Arc<Settings>
    where F: FnMut(&mut Settings) {
    cx.global::<SettingsGlobal>().0.write(change)
}

/// Applies a fallible `change` that writes its own document.
///
/// See [`SharedSettings::write_persisting`] for why this is separate from
/// [`write`] and when to prefer that one.
///
/// # Panics
///
/// See [`handle`].
pub(crate) fn write_persisting<F, T>(cx: &App, change: F) -> knot_core::Result<T>
    where F: FnOnce(&mut Settings) -> knot_core::Result<T> {
    cx.global::<SettingsGlobal>().0.write_persisting(change)
}
