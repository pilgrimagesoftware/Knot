//! Adopting preferences a *later* settings write put on disk.
//!
//! The window takes a copy of the settings surface when it opens
//! (`WorkspaceWindow::settings`), so every scalar it draws - the compact
//! tool-call mode, the send chord, the fonts - is the value that was current
//! when the window was built. Issue #238's read half: a preference changed
//! afterwards was written correctly and reached nothing already on screen,
//! which is why enabling `collapsed-tool-call-summary` appeared to do nothing
//! until the window was reopened.
//!
//! Only the preferences document is re-read. The roster and the other
//! collections stay as this window holds them - see
//! `Settings::reload_preferences` for why taking those from disk would be the
//! read-side twin of the write-side lost update this issue's other half
//! describes.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.

use gpui_kit::Context;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Re-read the preferences and repaint with them.
    ///
    /// Called from the settings window's persist path, which is a click -
    /// never from `render`, where the `.claude/rules/rust-structure.md` ban
    /// on I/O applies.
    ///
    /// A failed read leaves the window on the values it already had. There is
    /// nothing better to do with it: the preferences on disk are what failed
    /// to be read, and the copy in hand is the last set known to be good.
    pub(crate) fn adopt_preferences(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.settings.reload_preferences() {
            eprintln!("failed to re-read preferences: {error}");
            return;
        }
        cx.notify();
    }
}

/// The settings surface this window is drawing from.
///
/// Crate-visible under `cfg(test)` alone, the way `import_window` exposes its
/// view: `tests::settings_reach_open_windows` has to read the window's own
/// copy, because the whole defect was that this copy and the document
/// disagreed. An accessor the app itself could reach would invite the pattern
/// back.
#[cfg(test)]
impl WorkspaceWindow {
    pub(crate) fn settings_for_test(&self) -> &knot_core::Settings {
        &self.settings
    }
}
