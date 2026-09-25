//! The panel's visual vocabulary: the styling inputs a render is handed, and
//! the two mappings that decide what colour a thing takes - a tool call's
//! outline from its status, and a permission's risk from its wording.
//!
//! The mappings are named types rather than colours resolved on the spot, so
//! each is testable without a theme.

use gpui_kit::Hsla;

pub(super) const ERROR_COLOR: u32 = 0xEF4444;
pub(super) const SAFE_COLOR: u32 = 0x22C55E;
pub(super) const MUTED: u32 = 0x9CA3AF;

/// The panel's render-time styling inputs, grouped rather than passed as
/// two more positional parameters to `render_panel`.
#[derive(Clone, Debug)]
pub(crate) struct PanelStyle {
    pub(crate) permission_risk:    RiskLevel,
    /// The conversation's body text size, from `Settings`'
    /// `markdown_font_size`.
    pub(crate) markdown_font_size: gpui_kit::Pixels,
    /// The theme's monospace family, for tool output and diffs. A real
    /// registered family name is required: `font_family("monospace")` is
    /// not a family GPUI resolves, so it silently fell back to the body
    /// font and shell output rendered proportionally.
    pub(crate) mono_font_family:   gpui_kit::SharedString,
    /// The theme's proportional family, for the panel's own words about a
    /// call - named rather than inherited so a later change setting a
    /// card header monospace cannot sweep the status text along with the
    /// title.
    pub(crate) ui_font_family:     gpui_kit::SharedString,
    /// The user's title family, for Markdown headers in a rendered response.
    /// Named beside `ui_font_family` rather than read from the theme, which
    /// carries only the app-wide UI family - a header's face is the one thing
    /// a Markdown surface cannot inherit.
    pub(crate) title_font_family:  gpui_kit::SharedString,
    /// The theme's danger colour, for a failed tool call's outline.
    pub(crate) danger_color:       Hsla,
    /// The theme's info colour, for a pending or running call's outline.
    pub(crate) info_color:         Hsla,
    /// The theme's ordinary border, the neutral outline a completed call
    /// recedes to.
    pub(crate) border_color:       Hsla,
    /// The raised neutral surface a tool-call card sits on. From the
    /// platform's control background on macOS, so cards track the system
    /// appearance instead of a fixed near-black.
    pub(crate) card_color:         Hsla,
    /// The user prompt bubble's fill - the system accent on macOS - and the
    /// foreground picked to contrast it, so the prompt stays readable in
    /// either appearance and under any accent the user has chosen.
    pub(crate) prompt_color:       Hsla,
    pub(crate) prompt_foreground:  Hsla,
    /// Draw a turn's contiguous tool calls as one summary line instead of
    /// a card each, from `Settings`' `agent_panel_compact_tool_calls`.
    /// Off by default; a run the user has opened still draws its cards.
    pub(crate) compact_tool_calls: bool,
}

impl PanelStyle {
    /// The outline colour a tool call card's status calls for.
    pub(super) fn outline_color(&self, outline: CardOutline) -> Hsla {
        match outline {
            CardOutline::Danger => self.danger_color,
            CardOutline::Info => self.info_color,
            CardOutline::Neutral => self.border_color,
        }
    }
}

/// Which of `PanelStyle`'s three outline colours a tool call card takes.
/// Named rather than resolved directly to a colour so the mapping from
/// status is testable without a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardOutline {
    Danger,
    Info,
    Neutral,
}

/// A tool call's outline by status: danger for a failure, info while it
/// is still going, and the panel's neutral border once it is done.
///
/// Completed calls deliberately get no colour of their own - success is
/// the common case, and outlining every finished call leaves nothing
/// standing out. `status` is a plain wire string, so an unrecognized
/// value takes the neutral border rather than being treated as a failure.
pub(super) fn card_outline(status: &str) -> CardOutline {
    match status {
        "failed" => CardOutline::Danger,
        "pending" | "in_progress" => CardOutline::Info,
        _ => CardOutline::Neutral,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RiskLevel {
    Danger,
    Safe,
    Neutral,
}

pub(crate) fn permission_risk_level(value: &str, name: &str) -> RiskLevel {
    // Separators are folded to spaces so one keyword covers an id
    // (`agent-full-access`, Codex's counterpart to `bypassPermissions`)
    // and its display name ("Full access") alike.
    let value = format!("{value} {name}").to_ascii_lowercase()
                                         .replace(['-', '_'], " ");
    if ["bypass", "yolo", "danger", "full access"].iter()
                                                  .any(|word| value.contains(word))
    {
        RiskLevel::Danger
    }
    else if ["plan", "read"].iter().any(|word| value.contains(word)) {
        RiskLevel::Safe
    }
    else {
        RiskLevel::Neutral
    }
}

pub(crate) fn risk_color(risk: RiskLevel) -> Option<u32> {
    match risk {
        RiskLevel::Danger => Some(ERROR_COLOR),
        RiskLevel::Safe => Some(SAFE_COLOR),
        RiskLevel::Neutral => None,
    }
}
