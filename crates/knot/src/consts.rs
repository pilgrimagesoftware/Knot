//! Crate-wide constants for the `knot` binary.
//!
//! Every other crate in the workspace keeps its tunables in one `consts.rs`;
//! this one had none, so poll intervals, cache lifetimes and palette values
//! were literals scattered through the views that happened to use them -
//! which is how the same green ended up written three ways and two different
//! repaint cadences ended up describing the same spinner.
//!
//! A value belongs here when it is a *decision* (how often to poll, how
//! stale a cache may get, what colour a state is). A value stays inline when
//! it is part of one element's layout - a padding, a gap, a single width.

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------
use std::time::Duration;

use gpui_kit::Rgba;

/// How often the workspace window's poll wakes to ask whether anything that
/// changed off the main thread needs a repaint.
///
/// ~30 Hz. The poll itself is cheap - a few map lookups and an `Instant`
/// compare; it only calls `cx.notify()` when a predicate says something
/// actually moved.
pub(crate) const REPAINT_POLL_INTERVAL: Duration = Duration::from_millis(33);

/// Minimum gap between working-indicator repaints.
///
/// The spinner advances about five times a second, so repainting at the poll
/// rate would redraw the same frame five times over.
pub(crate) const WORKING_INDICATOR_MIN_REPAINT: Duration = Duration::from_millis(120);

/// How stale a cached `git diff --numstat` may get before the next render
/// asks for a fresh one.
///
/// `git` runs at most this often per agent no matter how often the window
/// repaints - see `workspace_window::sessions`.
pub(crate) const DIFF_STATS_MAX_AGE: Duration = Duration::from_secs(2);

/// How stale a pull request's fetched state may get before the Pull Requests
/// view asks for it again.
///
/// Far longer than a diff stat's two seconds: this is a network round trip
/// through `gh`, and a pull request's title and status change on a human
/// timescale rather than a keystroke's. A list of twenty rows therefore costs
/// twenty `gh` runs a minute while it is open, and none while it is not.
pub(crate) const PULL_REQUEST_STATE_MAX_AGE: Duration = Duration::from_secs(60);

/// How stale the answer to "can state be fetched at all" may be.
///
/// The same minute as the rows it gates, and one `gh auth status` against
/// their twenty `gh pr view`s. Re-asking at all is what makes signing in
/// take effect while the window stays open; re-asking faster would spend a
/// subprocess to notice something that only changes when the user goes and
/// does it.
pub(crate) const FORGE_PROBE_MAX_AGE: Duration = Duration::from_secs(60);

/// How often the settings window drains the native font panel's selections.
///
/// The panel is an AppKit window with no callback into GPUI, so its choice is
/// picked up by polling a shared slot. macOS-only, because the panel is.
#[cfg(target_os = "macos")]
pub(crate) const FONT_PANEL_POLL_INTERVAL: Duration = Duration::from_millis(300);

/// How often the settings window re-reads the MCP server's state.
///
/// The supervisor publishes over a channel on its own thread, which GPUI
/// cannot await, so the MCP tab follows a change by polling the mirror. Far
/// slower than the repaint poll: a lifecycle transition is a human-scale
/// event, and the tick only notifies when the state differs from the one
/// last drawn.
pub(crate) const MCP_STATE_POLL_INTERVAL: Duration = Duration::from_millis(500);

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------
//
// Fixed colours, not theme tokens: each one means the same thing in light and
// dark, and the theme's semantic colours do not cover them. Values are the
// Tailwind ramp the Swift reference used, so the two apps read alike.

/// Green: an agent that is idle, and a diff's added lines.
pub(crate) const COLOR_IDLE: u32 = 0x22C55E;

/// Orange: an agent that is working.
pub(crate) const COLOR_RUNNING: u32 = 0xF97316;

/// Blue: an agent awaiting input, and a diff's changed-file count.
pub(crate) const COLOR_INPUT: u32 = 0x3B82F6;

/// Red: an agent in error, and a diff's removed lines.
pub(crate) const COLOR_ERROR: u32 = 0xEF4444;

/// Grey: an agent that is stopped, and muted dashboard text.
pub(crate) const COLOR_STOPPED: u32 = 0x6B7280;

/// Muted text on a dashboard card.
pub(crate) const COLOR_CARD_MUTED: u32 = 0x888888;

/// Amber: a pull request that is open but cannot land - a conflict, a red
/// check, a draft. Distinct from [`COLOR_RUNNING`]'s orange, which is about
/// an agent rather than a pull request, and further from green so the two
/// open states are told apart at a glance.
pub(crate) const COLOR_PULL_REQUEST_BLOCKED: u32 = 0xEAB308;

/// Purple: a merged pull request. The one state with no counterpart in the
/// agent palette, and the colour GitHub itself uses for it.
pub(crate) const COLOR_PULL_REQUEST_MERGED: u32 = 0xA855F7;

/// How much of a pull request row's state colour reaches its background.
///
/// Low enough that the row's text keeps the theme's contrast in both light
/// and dark - the colour is a tint identifying the state, not a fill
/// competing with the title on top of it.
pub(crate) const PULL_REQUEST_ROW_TINT: f32 = 0.10;

/// How much reaches its border, where there is no text to stay legible
/// against and the colour does the identifying.
pub(crate) const PULL_REQUEST_ROW_BORDER_TINT: f32 = 0.55;

/// How wide the workspace name dialog is, in pixels.
///
/// Narrower than the dialog host's 448px default: the dialog holds one
/// single-line name field, and a box twice as wide as its contents reads as
/// an empty one.
pub(crate) const WORKSPACE_DIALOG_WIDTH: f32 = 360.;

/// The colour a workspace gets when it has none, or when the one it has
/// stored will not parse.
pub(crate) const COLOR_WORKSPACE_DEFAULT: u32 = 0x1B4FB2;

/// [`COLOR_WORKSPACE_DEFAULT`] in the `#rrggbb` form `Workspace::color_hex`
/// stores, so a new workspace and a workspace whose colour failed to parse
/// come out the same shade.
pub(crate) const COLOR_WORKSPACE_DEFAULT_HEX: &str = "#1B4FB2";

/// The workspace colour as a `Rgba`, for the parse-failure fallback.
pub(crate) fn workspace_default_color() -> Rgba {
    gpui_kit::rgb(COLOR_WORKSPACE_DEFAULT)
}

// ---------------------------------------------------------------------------
// Agent vocabulary
// ---------------------------------------------------------------------------

/// The avatar drawn for an agent that has none.
pub(crate) const DEFAULT_AGENT_AVATAR: &str = "🤖";

/// How much of a restored window's top edge has to land on a display for the
/// user to be able to grab it: the workspace title bar's height, so the whole
/// bar is reachable rather than a sliver of it.
///
/// A window restored onto a display that is no longer attached was placed
/// unclamped, which is how one ended up where it could not be dragged back.
pub(crate) const WINDOW_GRAB_STRIP_HEIGHT: f32 = 64.;

/// How much of a restored window's width has to overlap a display, so it
/// cannot be pushed to a few pixels at the screen edge and counted as visible.
pub(crate) const WINDOW_MIN_VISIBLE_WIDTH: f32 = 160.;
