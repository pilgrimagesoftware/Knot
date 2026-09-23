//! Getting a preference change to the windows already drawing the old value.
//!
//! The settings window writes `preferences.json` and every other window holds
//! its own copy of the settings surface, taken when it opened. Without a
//! delivery step the write reaches disk and nothing on screen - issue #238's
//! read half.
//!
//! Delivery goes through [`WindowRegistry`] rather than a new global with its
//! own observers: the registry already tracks which workspace windows are
//! open and already treats a dropped view as proof one closed, which is the
//! whole of the liveness question a broadcast has to ask.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.

use gpui_kit::App;

use crate::window_registry::WindowRegistry;

/// Hand every open workspace window the preferences that were just written.
///
/// Called after a successful write, not before: a window that adopted
/// preferences the write then failed to store would disagree with the disk
/// until it was reopened, which is the bug this module exists to fix, only
/// harder to see.
pub(crate) fn preferences_changed(cx: &mut App) {
    for view in WindowRegistry::workspace_views(cx) {
        view.update(cx, |window, cx| {
                window.adopt_preferences(cx);
            });
    }
}
