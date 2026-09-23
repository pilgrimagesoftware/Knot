//! Turns the stored appearance preference into the appearance the app paints.
//!
//! Contract: `openspec/specs/appearance-mode/spec.md`.
//!
//! The picker in the General pane used to write `Settings::appearance_mode`
//! and stop there - the theme came from the OS alone, at startup and on every
//! appearance flip, so Light and Dark changed the picker's label and nothing
//! else. Everything that decides what to paint now goes through [`resolve`],
//! which is the only place the preference and the OS are reconciled.

use gpui_kit::App;
use gpui_kit::Window;
use gpui_kit::WindowAppearance;
use gpui_kit::component::Theme;
use gpui_kit::component::ThemeMode;
use knot_core::AppearanceMode;

/// Which appearance the app paints, given the user's choice and what the OS
/// reports.
///
/// Exhaustive: adding a variant to [`AppearanceMode`] fails to compile here
/// rather than silently defaulting to the OS appearance.
///
/// `Auto` and `System` resolve identically. The Swift reference derives
/// `auto` from the terminal background colour's luminance
/// (`AppSettings.swift:143`); this port has no terminal background setting,
/// so there is no luminance to read. `Auto` stays because it is
/// `APPEARANCE_MODE_DEFAULT` and already sits in users' settings files -
/// dropping it would change what an existing document means on upgrade.
pub(crate) fn resolve(mode: AppearanceMode, system: WindowAppearance) -> ThemeMode {
    match mode {
        AppearanceMode::Light => ThemeMode::Light,
        AppearanceMode::Dark => ThemeMode::Dark,
        AppearanceMode::Auto | AppearanceMode::System => ThemeMode::from(system),
    }
}

/// The appearance mode in force, so the sites that repaint can reach it
/// without threading `Settings` through a window observer.
///
/// Installed at bootstrap from the loaded settings and rewritten when the
/// picker changes; one app-wide value, as in the reference, so two windows
/// cannot disagree about what the app looks like.
pub(crate) struct AppearancePreference(pub(crate) AppearanceMode);

impl gpui_kit::Global for AppearancePreference {}

/// The mode currently in force, or the default when bootstrap has not
/// installed one yet (a window opened during startup, and in tests).
pub(crate) fn current(cx: &App) -> AppearanceMode {
    cx.try_global::<AppearancePreference>()
      .map(|preference| preference.0)
      .unwrap_or_default()
}

/// Repaints to whatever [`resolve`] now returns.
///
/// `Theme::change` reloads the light or dark config wholesale, which discards
/// the system palette laid over it - hence the re-ingestion straight after,
/// in that order. Callers that have a window in hand pass it so it refreshes
/// with the theme already swapped.
pub(crate) fn apply(window: Option<&mut Window>, cx: &mut App) {
    let mode = resolve(current(cx), cx.window_appearance());
    Theme::change(mode, window, cx);
    crate::app_support::apply_system_palette(cx);
}

/// Records a new choice and repaints every open window.
///
/// Every window, not just the one the picker sits in: the theme is a global,
/// so a workspace window behind the settings window would otherwise keep its
/// old colors until something else happened to invalidate it.
pub(crate) fn set(mode: AppearanceMode, cx: &mut App) {
    cx.set_global(AppearancePreference(mode));
    apply(None, cx);
    cx.refresh_windows();
}

#[cfg(test)]
mod tests;
