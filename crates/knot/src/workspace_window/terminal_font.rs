//! The family the terminal draws in: the rule that resolves it, and the memo
//! that keeps that rule off the frame.
//!
//! Contract: `openspec/specs/terminal-rendering/spec.md`, "Drawing a terminal
//! frame does not enumerate installed fonts".
//!
//! Deciding whether the configured `terminal_font_name` is one the text
//! system can resolve means asking it which families exist, and on macOS that
//! is `CTFontCollectionCreateMatchingFontDescriptors` plus a `String` per
//! descriptor - 11.4ms measured over 181 families. The rule used to run from
//! `chrome::terminal_font_family` on every call, and two of its callers are
//! on the frame (`prepare_frame`'s resize and `render_grid`), so a terminal
//! that repaints for each keystroke it echoes spent ~23ms a frame deciding
//! something that changes only when the user edits a setting.
//!
//! So the rule is a pure function over the list, and the window holds one
//! [`TerminalFont`] that runs it when - and only when - the configured name
//! is not the one it already answered for. The same split
//! `settings_window::font::font_label` uses for the settings window's half of
//! this question, and testable for the same reason: no text system, no
//! window, no installed font.

use gpui_kit::App;
use gpui_kit::SharedString;

use crate::workspace_window::WorkspaceWindow;

/// The family the terminal falls back to when the configured one cannot be
/// resolved - embedded and registered by `app_support::apply_visual_identity`,
/// so it is always available.
///
/// The fallback matters more than it looks: an unresolvable name left to
/// GPUI's own lookup does not fail, it silently lands on the app's
/// proportional UI font, and a terminal grid in a proportional face is
/// unreadable rather than merely wrong.
pub(crate) const DEFAULT_FAMILY: &str = "JetBrains Mono";

/// The family to draw the terminal in: `requested` when `available` lists it,
/// otherwise [`DEFAULT_FAMILY`].
///
/// `available` is `cx.text_system().all_font_names()` - the platform's font
/// catalog plus whatever the app has embedded. Guards against a persisted
/// value the text system cannot resolve: an old default, a font since
/// uninstalled, or a name the macOS font panel accepts but GPUI's lookup does
/// not.
pub(crate) fn resolve(requested: &str, available: &[String]) -> SharedString {
    if available.iter().any(|name| name == requested) {
        SharedString::from(requested.to_string())
    }
    else {
        SharedString::from(DEFAULT_FAMILY)
    }
}

/// One window's answer to [`resolve`], and the name it was asked for.
///
/// Keyed on the requested name rather than cleared by an invalidation hook:
/// a hook is a second code path that every future way of changing settings
/// has to remember to call, which is the failure `settings_refresh` exists to
/// document. Comparing the name costs one string compare per frame and cannot
/// be forgotten.
pub(crate) struct TerminalFont {
    /// The configured name [`Self::resolved`] answers for, or `None` before
    /// the first frame has asked.
    requested: Option<String>,
    resolved:  SharedString,
}

impl Default for TerminalFont {
    fn default() -> Self {
        Self { requested: None,
               resolved:  SharedString::from(DEFAULT_FAMILY), }
    }
}

impl TerminalFont {
    /// The family for `requested`, running `available` only when this is not
    /// the name already answered for.
    pub(crate) fn family(&mut self, requested: &str, available: impl FnOnce() -> Vec<String>)
                         -> SharedString {
        if self.requested.as_deref() != Some(requested) {
            self.resolved = resolve(requested, &available());
            self.requested = Some(requested.to_string());
        }
        self.resolved.clone()
    }

    /// The family last resolved, without consulting anything.
    fn resolved(&self) -> SharedString {
        self.resolved.clone()
    }
}

impl WorkspaceWindow {
    /// Brings the memo up to date with the configured terminal font, so
    /// everything drawn or measured this frame reads one value.
    ///
    /// Called from `prepare_frame`, beside the window's other per-frame memo
    /// refreshes, and ahead of `resize_session_to_pane` - the frame's first
    /// reader.
    pub(super) fn refresh_terminal_font(&mut self, cx: &App) {
        self.terminal_font
            .family(&self.settings.terminal_font_name, || {
                cx.text_system().all_font_names()
            });
    }

    /// The family this frame's terminal draws and measures in.
    ///
    /// Answers from the memo alone. Event handlers - scroll, mouse - read it
    /// too and are safe to: an event can only reach the terminal pane after a
    /// frame has drawn that pane, and that frame refreshed this.
    pub(super) fn terminal_font_family(&self) -> SharedString {
        self.terminal_font.resolved()
    }
}
