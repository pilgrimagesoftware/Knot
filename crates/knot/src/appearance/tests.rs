//! Unit tests for [`super`]. No window here: resolution is a pure function of
//! the stored mode and what the OS reports, which is the point - the bug this
//! covers was that nothing outside the settings pane consulted the mode at
//! all.

use gpui_kit::WindowAppearance;
use gpui_kit::component::ThemeMode;
use knot_core::AppearanceMode;

use super::resolve;

/// Both spellings the platform has for each appearance: macOS reports the
/// vibrant variants under some materials, and a mode that only handled the
/// plain two would fall back to light in a vibrant dark window.
const LIGHT: [WindowAppearance; 2] = [WindowAppearance::Light, WindowAppearance::VibrantLight];
const DARK: [WindowAppearance; 2] = [WindowAppearance::Dark, WindowAppearance::VibrantDark];

/// The bug: picking Light on a Mac in dark mode left the app dark.
#[test]
fn an_explicit_choice_overrides_the_os() {
    for system in DARK {
        assert_eq!(resolve(AppearanceMode::Light, system), ThemeMode::Light);
    }
    for system in LIGHT {
        assert_eq!(resolve(AppearanceMode::Dark, system), ThemeMode::Dark);
    }
}

/// An explicit choice is not a one-way door: it must hold whichever way the
/// OS is already set, or it would look like it worked only on a mismatch.
#[test]
fn an_explicit_choice_holds_when_the_os_agrees() {
    for system in LIGHT {
        assert_eq!(resolve(AppearanceMode::Light, system), ThemeMode::Light);
    }
    for system in DARK {
        assert_eq!(resolve(AppearanceMode::Dark, system), ThemeMode::Dark);
    }
}

#[test]
fn system_follows_the_os() {
    for system in LIGHT {
        assert_eq!(resolve(AppearanceMode::System, system), ThemeMode::Light);
    }
    for system in DARK {
        assert_eq!(resolve(AppearanceMode::System, system), ThemeMode::Dark);
    }
}

/// `Auto` has no terminal background to derive from in this port, so it
/// follows the OS exactly as `System` does. Asserted rather than left
/// implicit: it is the default mode, so it is what most users get.
#[test]
fn auto_follows_the_os_like_system() {
    for system in LIGHT.into_iter().chain(DARK) {
        assert_eq!(resolve(AppearanceMode::Auto, system),
                   resolve(AppearanceMode::System, system));
    }
}

/// Driven off `AppearanceMode::ALL`, so a variant added later is covered here
/// without anyone remembering to extend the test: each mode either defers to
/// the OS for both appearances or ignores it for both. A mode that tracked the
/// OS only half the time would be a mode nobody can predict.
#[test]
fn each_mode_either_defers_to_the_os_or_ignores_it() {
    for mode in AppearanceMode::ALL {
        let defers_on_light = LIGHT.iter()
                                   .all(|system| resolve(*mode, *system) == ThemeMode::Light);
        let defers_on_dark = DARK.iter()
                                 .all(|system| resolve(*mode, *system) == ThemeMode::Dark);
        let ignores_the_os = LIGHT.iter()
                                  .chain(&DARK)
                                  .map(|system| resolve(*mode, *system))
                                  .collect::<std::collections::HashSet<_>>()
                                  .len()
                             == 1;

        assert!((defers_on_light && defers_on_dark) || ignores_the_os,
                "{mode:?} tracks the OS inconsistently");
    }
}

/// The default must keep deferring to the OS: it is what a settings file
/// written before this change resolves to, and a relaunch must not repaint
/// such a user into a scheme they never chose.
#[test]
fn the_default_mode_defers_to_the_os() {
    for system in LIGHT {
        assert_eq!(resolve(AppearanceMode::default(), system), ThemeMode::Light);
    }
    for system in DARK {
        assert_eq!(resolve(AppearanceMode::default(), system), ThemeMode::Dark);
    }
}
