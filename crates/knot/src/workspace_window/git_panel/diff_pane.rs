//! The diff for the selected row.
//!
//! Lines are drawn through `gpui_kit::list`, the same virtualization the
//! conversation panel uses, so the work one frame does is bounded by the
//! viewport rather than by the size of the diff. Swift drew every line of
//! every hunk, which it could afford because SwiftUI does not rebuild the
//! whole element tree per frame.
//!
//! Within a line, the run-per-style shape `terminal_view` uses: an `h_flex`
//! per line, a `div` per styled run.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`.

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    Context, InteractiveElement, IntoElement, ListAlignment, ListState, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px, rgb,
};
use knot_git::{DiffLine, FileDiff, LineKind};

use crate::consts;
use crate::git_panel::state::DiffOutcome;
use crate::workspace_window::WorkspaceWindow;

/// Width of each line-number gutter. Wide enough for five digits, which
/// covers any file a person reads a diff of.
const GUTTER_WIDTH: f32 = 44.;

/// One flattened diff line, hunk headers included, ready to draw.
///
/// Flattened out of the hunks once per selection rather than per frame: the
/// list asks for row `n`, and walking the hunk tree to find it every time is
/// how a virtualized list ends up costing what the whole diff costs.
pub(super) struct DiffRow {
    pub(super) kind: LineKind,
    pub(super) text: String,
    pub(super) old:  Option<u32>,
    pub(super) new:  Option<u32>,
}

impl WorkspaceWindow {
    /// The diff pane for the current selection.
    pub(super) fn git_diff_pane(&mut self, agent: uuid::Uuid, diff: Option<DiffOutcome>,
                                _window: &mut Window, cx: &mut Context<Self>)
                                -> gpui_kit::AnyElement {
        let Some(outcome) = diff
        else {
            // No selection at all is a different state from a selection whose
            // diff has not landed; both read as "nothing to show yet" here,
            // and the invitation is the honest caption for either.
            return empty_pane(knot_core::l10n::t("git_panel.select_a_file"), cx);
        };

        match outcome {
            DiffOutcome::Failed(reason) => {
                empty_pane(knot_core::l10n::t_with("git_panel.diff_failed", &[("reason", &reason)]),
                           cx)
            }
            DiffOutcome::Absent => empty_pane(knot_core::l10n::t("git_panel.no_changes"), cx),
            DiffOutcome::Loaded(diff) => self.loaded_diff_pane(agent, *diff, cx),
        }
    }

    fn loaded_diff_pane(&mut self, agent: uuid::Uuid, diff: FileDiff, cx: &mut Context<Self>)
                        -> gpui_kit::AnyElement {
        if diff.binary {
            return empty_pane(knot_core::l10n::t("git_panel.binary_file"), cx);
        }
        if diff.hunks.is_empty() {
            return empty_pane(knot_core::l10n::t("git_panel.no_changes"), cx);
        }

        let header = diff_header(&diff);
        let (rows, truncated) = flatten(&diff);
        let row_count = rows.len();

        let list = self.git_diff_list(agent, row_count);
        let shared = std::sync::Arc::new(rows);

        v_flex().flex_1()
                .min_h_0()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(header)
                .children(truncated.map(|shown| {
                                       div().px_3()
                     .py_1()
                     .text_xs()
                     .text_color(cx.theme().muted_foreground)
                     .child(knot_core::l10n::t_with("git_panel.diff_truncated",
                                                    &[("count", &shown.to_string())]))
                                   }))
                .child(div().id("git-diff-lines")
                            .flex_1()
                            .min_h_0()
                            .overflow_x_scroll()
                            .child(gpui_kit::list(list, move |index, _window, cx| {
                                       render_line(&shared[index], cx)
                                   }).size_full()))
                .into_any_element()
    }

    /// The list state for the diff, reconciled to `row_count`.
    ///
    /// One per agent rather than per selection: only one diff is on screen at
    /// a time, so a second entry would be a leak, not a cache.
    ///
    /// `reset_with_uniform_height` rather than `splice`, for two reasons. A
    /// count change here means different content - another file, or the same
    /// file after a stage - so no measured height from the previous one is
    /// worth keeping, which is what `reset` means and `splice` does not. And
    /// the uniform hint keeps the summary measurable: an unmeasured row
    /// summarises as zero height with an unknown-height flag that ORs up the
    /// tree, so one unmeasured row anywhere makes anything derived from total
    /// content height wrong - silently, for a scrollbar, since those
    /// accessors return a plausible number rather than `None`. Diff rows are
    /// uniform, so the hint fits them.
    ///
    /// Deliberately not `measure_all`: it re-measures every row on every
    /// splice, which is the cost virtualizing exists to avoid.
    fn git_diff_list(&mut self, agent: uuid::Uuid, row_count: usize) -> ListState {
        let known = self.git_diff_row_counts.get(&agent).copied();
        let list = self.git_diff_lists.entry(agent).or_insert_with(|| {
                                                       ListState::new(0,
                                                         ListAlignment::Top,
                                                         px(consts::GIT_DIFF_LIST_OVERDRAW))
                                                   });

        if known != Some(row_count) {
            list.reset_with_uniform_height(row_count, px(consts::GIT_DIFF_LINE_HEIGHT));
            self.git_diff_row_counts.insert(agent, row_count);
        }

        list.clone()
    }

    /// Drops the diff list for `agent`, so a closed panel leaves nothing
    /// behind.
    pub(in crate::workspace_window) fn forget_git_diff_list(&mut self, agent: uuid::Uuid) {
        self.git_diff_lists.remove(&agent);
        self.git_diff_row_counts.remove(&agent);
    }
}

/// The fixed palette, for the header's `+N` / `-N` figures only.
///
/// These stay on `consts` rather than the theme because the agent header
/// directly above the panel draws the same two figures through
/// `app_state::diff_stats_row`, which uses this palette app-wide. Two
/// different greens for the same number, a few pixels apart, is worse than
/// one imperfect one - and unlike the diff body, this is a two-character
/// accent rather than something anybody reads a screenful of.
fn added_color() -> gpui_kit::Hsla {
    rgb(consts::COLOR_IDLE).into()
}

fn removed_color() -> gpui_kit::Hsla {
    rgb(consts::COLOR_ERROR).into()
}

/// The path, and the counts that are greater than zero.
fn diff_header(diff: &FileDiff) -> gpui_kit::AnyElement {
    let additions = diff.additions();
    let deletions = diff.deletions();

    h_flex().w_full()
            .min_w_0()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .py_2()
            .child(div().flex_1()
                        .min_w_0()
                        .text_xs()
                        .font_medium()
                        .truncate()
                        .child(diff.path.display().to_string()))
            .child(
                   h_flex().flex_shrink_0()
                           .gap_2()
                           .children((additions > 0).then(|| {
                                                        div().text_xs()
                                              .text_color(added_color())
                                              .child(knot_core::l10n::t_with(
                    "git_panel.additions",
                    &[("count", &additions.to_string())],
                ))
                                                    }))
                           .children((deletions > 0).then(|| {
                                                        div().text_xs()
                                              .text_color(removed_color())
                                              .child(knot_core::l10n::t_with(
                    "git_panel.deletions",
                    &[("count", &deletions.to_string())],
                ))
                                                    })),
    )
            .into_any_element()
}

/// Flattens the hunks into drawable rows, capped.
///
/// The cap bounds what is held, not what is drawn - drawing is already
/// virtualized. A generated file of several hundred thousand lines should not
/// sit in the cache per selected row. Returns the shown count when the diff
/// was cut short, so the pane can say so rather than presenting part of a
/// file as all of it.
fn flatten(diff: &FileDiff) -> (Vec<DiffRow>, Option<usize>) {
    let mut rows = Vec::new();

    for hunk in &diff.hunks {
        for line in &hunk.lines {
            if rows.len() >= consts::GIT_DIFF_MAX_LINES {
                let shown = rows.len();
                return (rows, Some(shown));
            }
            rows.push(row_of(line));
        }
    }

    (rows, None)
}

fn row_of(line: &DiffLine) -> DiffRow {
    DiffRow { kind: line.kind,
              text: line.text.clone(),
              old:  line.old_lineno,
              new:  line.new_lineno, }
}

/// One line: two gutters and the text, none of it wrapping.
///
/// A wrapped diff line breaks the column alignment that makes a diff readable
/// and desynchronises it from its gutter, which is why the pane scrolls
/// horizontally instead.
fn render_line(row: &DiffRow, cx: &mut gpui_kit::App) -> gpui_kit::AnyElement {
    let (background, foreground) = line_colors(row.kind, cx);

    h_flex().w_full()
            .px_2()
            .bg(background)
            .child(gutter(row.old, cx))
            .child(gutter(row.new, cx))
            .child(div().flex_1()
                        .whitespace_nowrap()
                        .text_xs()
                        .font_family(mono_family(cx))
                        .text_color(foreground)
                        .child(format!("{}{}", prefix(row.kind), row.text)))
            .into_any_element()
}

fn gutter(number: Option<u32>, cx: &mut gpui_kit::App) -> gpui_kit::AnyElement {
    div().w(px(GUTTER_WIDTH))
         .flex_shrink_0()
         .text_xs()
         .font_family(mono_family(cx))
         .text_color(muted(cx))
         .child(number.map(|n| n.to_string()).unwrap_or_default())
         .into_any_element()
}

/// The character git itself puts in front of the line, so a copied line reads
/// as a diff line rather than as content.
fn prefix(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Addition => "+",
        LineKind::Deletion => "-",
        LineKind::Context => " ",
        LineKind::Header | LineKind::HunkHeader => "",
    }
}

/// The background wash and text colour for one line.
///
/// Both come from theme tokens rather than the fixed palette in `consts`.
/// That palette is documented as meaning the same thing in light and dark,
/// and it does - for a status dot or a stat figure, which is what it was
/// written for. A diff pane is not that: it is a wall of body text, and the
/// palette's green is 2.28:1 on white, below even the threshold for large
/// text. The theme's tokens are the ones picked to be legible against the
/// theme's own background, which is exactly the question here.
///
/// The `+`/`-` prefix carries the same distinction as the colour, so a line's
/// classification survives when the colour does not - a diff read without
/// colour vision is still a diff.
fn line_colors(kind: LineKind, cx: &mut gpui_kit::App) -> (gpui_kit::Hsla, gpui_kit::Hsla) {
    let theme = cx.theme();
    match kind {
        LineKind::Addition => (tint(theme.success), theme.success),
        LineKind::Deletion => (tint(theme.danger), theme.danger),
        LineKind::HunkHeader | LineKind::Header => (tint(theme.info), theme.info),
        LineKind::Context => (gpui_kit::transparent_black(), theme.foreground),
    }
}

/// A background wash of the line's own colour, faint enough to read through.
fn tint(color: gpui_kit::Hsla) -> gpui_kit::Hsla {
    gpui_kit::Hsla { a: 0.15, ..color }
}

fn empty_pane(text: String, cx: &mut Context<WorkspaceWindow>) -> gpui_kit::AnyElement {
    v_flex().flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .border_t_1()
            .border_color(cx.theme().border)
            .p_4()
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(text))
            .into_any_element()
}

/// The theme's monospace family - a diff's columns only line up in one.
fn mono_family(cx: &gpui_kit::App) -> SharedString {
    cx.theme().font_family.clone()
}

fn muted(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    cx.theme().muted_foreground
}
