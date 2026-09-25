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

/// How stale a cached `git diff --numstat` may get before the next render
/// asks for a fresh one.
///
/// `git` runs at most this often per agent no matter how often the window
/// repaints - see `workspace_window::sessions`.
pub(crate) const DIFF_STATS_MAX_AGE: Duration = Duration::from_secs(2);

/// How stale the git panel's working-tree status may get before the next
/// render asks for a fresh one.
///
/// Shorter than a diff stat's, because the panel is what the user acts
/// through: a stage they just made must appear promptly, and the watch that
/// would otherwise tell us is paused for exactly that window.
pub(crate) const GIT_STATUS_MAX_AGE: Duration = Duration::from_millis(500);

/// How stale a shown file diff may get before it is read again.
///
/// Long, because a diff is re-read when the selection changes or the status
/// is invalidated, not on a clock - this is a backstop against a diff that
/// somehow outlives both, not the mechanism that keeps it fresh.
pub(crate) const GIT_DIFF_MAX_AGE: Duration = Duration::from_secs(30);

/// The git panel's width when it opens, and the bounds a drag may take it to.
///
/// Carried from the Swift panel, which clamped the same way. Not persisted:
/// a reopened panel starts at the default again.
pub(crate) const GIT_PANEL_DEFAULT_WIDTH: f32 = 500.;
pub(crate) const GIT_PANEL_MIN_WIDTH: f32 = 350.;
pub(crate) const GIT_PANEL_MAX_WIDTH: f32 = 800.;

/// The artifact panel's width when it opens, and the bounds a drag may take
/// it to.
///
/// The same three values the git panel uses, and deliberately so: the two are
/// siblings in the same resizable group, and a user who has learned how wide
/// one goes has learned the other. Carried from `ArtifactPanelView`'s own
/// `max(350, min(800, ...))`. Not persisted, for the reason the git panel's
/// are not.
pub(crate) const ARTIFACT_PANEL_DEFAULT_WIDTH: f32 = 500.;
pub(crate) const ARTIFACT_PANEL_MIN_WIDTH: f32 = 350.;
pub(crate) const ARTIFACT_PANEL_MAX_WIDTH: f32 = 800.;

/// How the artifact panel's height divides between its two sections, and how
/// far a drag on the divider may take it.
///
/// The clamp keeps either section from being dragged away entirely: at the
/// limit the smaller one still shows content rather than collapsing to its
/// header, which is what the collapse chevron is for and is a different
/// gesture. Carried from `ArtifactPanelView.sectionDivider`.
pub(crate) const ARTIFACT_PANEL_DEFAULT_SPLIT: f32 = 0.5;
pub(crate) const ARTIFACT_PANEL_MIN_SPLIT: f32 = 0.15;
pub(crate) const ARTIFACT_PANEL_MAX_SPLIT: f32 = 0.85;

/// A collapsed artifact section's height: its header and nothing else.
pub(crate) const ARTIFACT_SECTION_HEADER_HEIGHT: f32 = 34.;

/// The draggable divider between the two artifact sections.
///
/// Counted out of the height the sections share, so the two plus this equal
/// the section area exactly at every split.
pub(crate) const ARTIFACT_PANEL_DIVIDER_HEIGHT: f32 = 4.;

/// The least width the content pane keeps while side panels are open.
///
/// Both side panels have a 350pt minimum of their own, which on a narrow
/// window would otherwise leave the conversation nothing. This does not apply
/// while the artifact panel is expanded: that state takes the content pane's
/// width deliberately, and is the one case both focus requirements account
/// for.
pub(crate) const CONTENT_PANE_MIN_WIDTH: f32 = 360.;

/// The most diff lines the panel will hold and draw for one file.
///
/// Drawing is virtualized, so this is not what bounds the frame - it bounds
/// what `parse_diff` materializes into the cache. A generated file of several
/// hundred thousand lines should not be held in memory per selected row.
pub(crate) const GIT_DIFF_MAX_LINES: usize = 20_000;

/// How much space above and below the diff viewport the list measures, so
/// scrolling a diff does not pop lines in at the edges. The conversation
/// panel's own overdraw, for the same reason.
pub(crate) const GIT_DIFF_LIST_OVERDRAW: f32 = 400.;

/// The height hint every diff row starts with, before it has been measured.
///
/// A virtualized list summarises an unmeasured row as zero height and flags
/// the whole summary unknown, so anything derived from total content height -
/// a scrollbar thumb above all - is wrong until every row has been drawn.
/// Seeding a hint fixes that from the first frame, and real heights replace
/// it as rows render.
///
/// The value is the computed line box, not an estimate: `text_xs` is
/// `rems(0.75)` against the default 16px rem, so 12px; the default
/// `line_height` is `phi` *relative to the font size*, so 12 x 1.618034 =
/// 19.416, rounded to 19. A diff row carries horizontal padding only, and no
/// border, so the line box is the whole row.
///
/// One hint works here and would not for a conversation: diff rows are
/// uniform - one line each, one text size, no wrapping.
pub(crate) const GIT_DIFF_LINE_HEIGHT: f32 = 19.;

/// How stale a pull request's fetched state may get before the Pull Requests
/// view asks for it again.
///
/// Far longer than a diff stat's two seconds: this is a network round trip
/// through `gh`, and a pull request's title and status change on a human
/// timescale rather than a keystroke's. A list of twenty rows therefore costs
/// twenty `gh` runs a minute while it is open, and none while it is not.
pub(crate) const PULL_REQUEST_STATE_MAX_AGE: Duration = Duration::from_secs(60);

/// How long a merged pull request stays in the list after it merged.
///
/// Measured against the merge time the forge reports, not against when Knot
/// first saw the URL: a sighting says nothing about when the pull request
/// landed. A day is long enough that work merged this morning is still there
/// when the user looks after lunch, and short enough that the list is what is
/// outstanding rather than a lifetime tally.
///
/// A constant rather than a setting: a settings row, a persisted field and a
/// migration are a lot to spend on a number nobody has yet asked to change,
/// and this forecloses none of them.
pub(crate) const PULL_REQUEST_MERGED_RETENTION: Duration = Duration::from_secs(24 * 60 * 60);

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

/// Orange: an agent that is working, and an open pull request that cannot
/// land without work - a conflict, a red check, a draft, a required review.
pub(crate) const COLOR_RUNNING: u32 = 0xF97316;

/// Blue: an agent awaiting input, a diff's changed-file count, and an open
/// pull request whose checks are still running.
pub(crate) const COLOR_INPUT: u32 = 0x3B82F6;

/// Red: an agent in error, and a diff's removed lines.
pub(crate) const COLOR_ERROR: u32 = 0xEF4444;

/// Grey: an agent that is stopped, and muted dashboard text.
pub(crate) const COLOR_STOPPED: u32 = 0x6B7280;

/// Muted text on a dashboard card.
pub(crate) const COLOR_CARD_MUTED: u32 = 0x888888;

/// Yellow: an open pull request whose branch is behind its base. Updating
/// the branch is all it needs, so it sits between green and orange.
pub(crate) const COLOR_PULL_REQUEST_BEHIND: u32 = 0xEAB308;

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
