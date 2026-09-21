//! The compact tool-call summary line: one row standing in for a run of
//! contiguous tool calls when `agent-panel-compact-tool-calls` is on.
//!
//! Draws only - the run and its counts are derived by
//! [`crate::panel_state::PanelState::tool_run_summary`], and the cards
//! themselves stay in panel state untouched. Clicking the line opens the
//! run's calls, which is the inspection path the spec requires compact
//! mode to keep.
//!
//! Contract: `openspec/specs/collapsed-tool-call-summary/spec.md`.

use knot_core::l10n::{t, t_with};

use super::*;
use crate::panel_state::ToolRunSummary;

/// What compact mode makes of one message row.
///
/// Compact mode never changes the row model: every message keeps its own
/// list row, so the virtualizer's measured heights and the reader's scroll
/// position survive a mode switch untouched. A covered row simply draws
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CompactRow {
    /// Draw the message as it normally renders.
    Message,
    /// Draw this run's summary line in place of its first tool call.
    Summary,
    /// Draw nothing: the summary above already accounts for this call.
    Covered,
}

/// Which of the three `message_index` is, given the mode and the run's
/// open/closed state. Everything is `Message` when compact mode is off,
/// and a run the user has opened draws its cards as usual.
///
/// Takes the flag rather than the whole `PanelStyle` - the decision needs
/// nothing else from it, and a bool is testable without a theme.
pub(super) fn compact_row(message_index: usize, state: &PanelState, compact: bool) -> CompactRow {
    if !compact {
        return CompactRow::Message;
    }
    let expanded = state.tool_run_head(message_index)
                        .is_some_and(|head| state.is_tool_run_expanded(head));
    if expanded {
        return CompactRow::Message;
    }
    if state.starts_tool_run(message_index) {
        CompactRow::Summary
    }
    else if state.continues_tool_run(message_index) {
        CompactRow::Covered
    }
    else {
        CompactRow::Message
    }
}

/// One run's summary line. The whole row is the hit target that expands
/// the run back into cards, matching the tool-call header's own "the
/// whole header toggles" behaviour rather than introducing a second
/// interaction idiom.
pub(super) fn render_tool_run_summary(summary: ToolRunSummary, head_id: String,
                                      style: &PanelStyle, on_expand: Rc<dyn Fn(String)>)
                                      -> impl IntoElement {
    let element_key = tool_call::element_id(&head_id);
    let icon = if summary.running {
        IconName::LoaderCircle
    }
    else if summary.failed > 0 {
        IconName::CircleX
    }
    else {
        IconName::Check
    };
    let icon_color = if summary.failed > 0 {
        style.danger_color
    }
    else if summary.running {
        style.info_color
    }
    else {
        rgb(MUTED).into()
    };
    h_flex().id(("panel-tool-run-summary", element_key))
            .w_full()
            .min_w_0()
            .gap_2()
            .items_center()
            .cursor_pointer()
            .on_click(move |_: &ClickEvent, _, _| on_expand(head_id.clone()))
            .child(Icon::new(IconName::ChevronRight).xsmall()
                                                    .text_color(rgb(MUTED)))
            .child(Icon::new(icon).xsmall().text_color(icon_color))
            .child(div().flex_1()
                        .min_w_0()
                        .font_family(style.ui_font_family.clone())
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(summary_text(summary)))
}

/// The line's words: the call count always, then whichever activity
/// counts the run actually has, then a failure count when there is one.
/// A count the tool metadata does not support is omitted rather than
/// shown as zero.
pub(super) fn summary_text(summary: ToolRunSummary) -> String {
    let mut parts = vec![count_phrase(summary.calls, "panel.summary.calls")];
    if summary.files_edited > 0 {
        parts.push(count_phrase(summary.files_edited, "panel.summary.files_edited"));
    }
    if summary.files_read > 0 {
        parts.push(count_phrase(summary.files_read, "panel.summary.files_read"));
    }
    if summary.commands_run > 0 {
        parts.push(count_phrase(summary.commands_run, "panel.summary.commands_run"));
    }
    if summary.failed > 0 {
        parts.push(count_phrase(summary.failed, "panel.summary.failures"));
    }
    let body = parts.join(&t("panel.summary.separator"));
    if summary.running {
        format!("{body}{}", t("panel.summary.running_suffix"))
    }
    else {
        body
    }
}

/// `n` rendered through the catalogue entry its plurality calls for.
///
/// Each phrase is one entry rather than a noun glued to a number at the
/// call site, so the word order around the count stays the translator's
/// choice - the reasoning `knot_core::l10n::t_with` documents.
fn count_phrase(n: usize, key: &str) -> String {
    let suffix = if n == 1 { "_one" } else { "_other" };
    t_with(&format!("{key}{suffix}"), &[("count", &n.to_string())])
}
