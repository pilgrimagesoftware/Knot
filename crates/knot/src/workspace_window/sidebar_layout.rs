//! The sidebar's width: where the current one comes from, when it makes the
//! sidebar compact, and how a finished drag is persisted.
//!
//! Contract: `openspec/specs/agent-list-ui/spec.md` - "The sidebar's width is
//! set by a divider the user drags" and "A dragged width outlives the window".
//!
//! One module rather than three call sites so the breakpoint is read in
//! exactly one place: the agent row, the dashboard row, the sidebar's title
//! bar and the new-agent button all take the `bool` this produces, and cannot
//! disagree about where compact begins.

use gpui_kit::App;
use knot_core::consts::{SIDEBAR_COMPACT_BREAKPOINT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN};

use crate::workspace_window::WorkspaceWindow;

/// Whether a sidebar `width` draws the compact layout.
///
/// Exactly at the breakpoint is full width - the breakpoint is the narrowest
/// width that still fits the full row, not the widest that does not.
pub(crate) fn sidebar_is_compact(width: f64) -> bool {
    width < SIDEBAR_COMPACT_BREAKPOINT
}

impl WorkspaceWindow {
    /// The sidebar's width right now: what the resizable state has measured,
    /// or the persisted width until it has measured anything.
    ///
    /// The state seeds every panel at a placeholder size and replaces it on
    /// the first prepaint, so the first frame of a window would otherwise
    /// read as narrower than the sidebar will ever be drawn. The panel's own
    /// `size_range` floors a measured width at [`SIDEBAR_WIDTH_MIN`], which
    /// is what makes anything below it recognisable as the placeholder.
    pub(crate) fn sidebar_width(&self, cx: &App) -> f64 {
        let measured = self.sidebar_resize
                           .read(cx)
                           .sizes()
                           .first()
                           .map(|width| f64::from(f32::from(*width)));
        match measured {
            Some(width) if width >= SIDEBAR_WIDTH_MIN => width,
            _ => self.settings.sidebar_width,
        }
    }

    /// Record a width the user just finished dragging to.
    ///
    /// Re-reads the store from disk first, as `agent_editor` and the bench
    /// menu do: this window's snapshot went stale the moment another window
    /// persisted, and writing it back would silently revert that window's
    /// edits. The window's own snapshot is updated too, so the next render
    /// agrees with what is on disk.
    pub(crate) fn persist_sidebar_width(&mut self, width: f64) {
        let mut settings = knot_core::Settings::load().unwrap_or_else(|_| self.settings.clone());
        settings.sidebar_width = width.clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX);
        if let Err(error) = settings.persist_preferences() {
            self.error = Some(knot_core::l10n::t_with("sidebar.width_error",
                                                      &[("error", &error.to_string())]));
            return;
        }
        self.settings = settings;
    }
}
