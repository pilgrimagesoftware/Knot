//! Installing a [`Resolved`] set into gpui's keymap.
//!
//! gpui's keymap has no removal short of `clear_key_bindings`, which would
//! also erase the bindings `gpui_kit::init` installs for its own components.
//! A binding is moved instead by binding `gpui::Unbind(action name)` on the
//! old chord, which hides exactly that chord-and-action pair from every
//! earlier binding, and then binding the action on the new chord. The keymap
//! grows by two entries per change, which a user-driven setting never makes
//! matter.

use gpui_kit::App;
use gpui_kit::Global;
use gpui_kit::KeyBinding;
use gpui_kit::Unbind;

use crate::keymap::Resolved;

/// What [`apply`] last installed, as `(chord, action name)` pairs: the old
/// side of the next change.
#[derive(Default)]
struct AppliedKeymap(Vec<(String, &'static str)>);

impl Global for AppliedKeymap {}

/// Makes `resolved` the configurable set in effect, touching only the
/// bindings that differ from the last call. The first call binds all of
/// them.
///
/// The menu bar does not follow on its own - a `Menu` is a snapshot of the
/// keymap taken when it is set - so a caller changing the Command Center's
/// chord rebuilds the menus afterwards.
pub(crate) fn apply(resolved: &Resolved, cx: &mut App) {
    let wanted: Vec<(String, &'static str, KeyBinding)> =
        resolved.bindings()
                .iter()
                .map(|configured| {
                    let binding = configured.key_binding();
                    (configured.chord.to_gpui(), binding.action().name(), binding)
                })
                .collect();
    let previous = cx.try_global::<AppliedKeymap>()
                     .map(|applied| applied.0.clone())
                     .unwrap_or_default();

    let mut changes = Vec::new();
    for (chord, name) in &previous {
        if !wanted.iter().any(|(c, n, _)| c == chord && n == name) {
            changes.push(KeyBinding::new(chord, Unbind((*name).into()), None));
        }
    }
    let mut installed = Vec::with_capacity(wanted.len());
    for (chord, name, binding) in wanted {
        if !previous.iter().any(|(c, n)| *c == chord && *n == name) {
            changes.push(binding);
        }
        installed.push((chord, name));
    }
    cx.bind_keys(changes);
    cx.set_global(AppliedKeymap(installed));
}

/// [`apply`], then rebuilds the menu bar from its current snapshot so every
/// item on a configurable shortcut - Window > Command Center, the View
/// menu's - shows the chord now in effect.
pub(crate) fn apply_and_refresh_menus(resolved: &Resolved, cx: &mut App) {
    apply(resolved, cx);
    let snapshot = cx.try_global::<crate::menu_bar::MenuBarState>()
                     .map(|state| state.snapshot.clone())
                     .unwrap_or_default();
    crate::app_bootstrap::set_app_menus(&snapshot, cx);
}
