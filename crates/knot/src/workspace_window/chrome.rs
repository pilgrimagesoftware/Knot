//! Small presentational helpers for the workspace window's chrome: the
//! agent-row detail lines, the terminal font/metrics lookups, and the
//! context-usage indicator.
//!
//! None of these touch [`super::WorkspaceWindow`] state - they take what
//! they need and return an element or a value, which is what makes them
//! separable from the window itself and unit-testable as they stand.

use gpui_kit::App;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::PathBuilder;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::canvas;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::point;
use gpui_kit::px;
use uuid::Uuid;

use crate::consts;
use crate::workspace_window::DetailLineSize;

/// One labelled detail line on an agent row: a leading icon saying what the
/// line is, then the text.
///
/// Without the icon a line is a bare string whose meaning has to be inferred
/// from its content, which fails exactly when it matters - a status that
/// mentions a path, a folder named after a person.
///
/// `items_start`, not `items_center`: the persona line has no
/// `whitespace_nowrap`, so a name like "DevOps Troubleshooter" wraps onto a
/// second line, and a centred icon floats into the gap between the two. On a
/// single-line row the two are indistinguishable.
pub(crate) fn detail_line(icon: gpui_kit::assets::IconName, text: String, size: DetailLineSize,
                          font_family: String, font_size: gpui_kit::Pixels, cx: &App)
                          -> gpui_kit::AnyElement {
    let muted = cx.theme().muted_foreground;
    let icon = match size {
        DetailLineSize::Small => Icon::new(icon).xsmall(),
        DetailLineSize::Body => Icon::new(icon).small(),
    };
    let line = div().font_family(font_family)
                    .text_size(font_size)
                    .text_color(muted);
    let line = match size {
        DetailLineSize::Small => line.text_xs(),
        // The status and folder lines truncate rather than wrap, which is
        // what keeps a long path from growing the row.
        DetailLineSize::Body => line.overflow_hidden().whitespace_nowrap().text_ellipsis(),
    };
    h_flex().w_full()
            .min_w_0()
            .gap_1()
            .items_start()
            .child(icon.text_color(muted).flex_shrink_0())
            .child(line.min_w_0().child(text))
            .into_any_element()
}

/// Whether an agent of this type runs a terminal process of its own.
///
/// Under ACP-only launch only shell agents do; every other type reaches
/// its agent through an adapter subprocess owned by the panel session.
/// This is what scopes `agent-lifecycle`'s exit-driven removal: a shell
/// agent whose process exits is removed, an ACP agent whose adapter exits
/// is not.
pub(crate) fn runs_a_terminal_process(agent_type: &str) -> bool {
    agent_type == consts::SHELL_AGENT_TYPE
}

/// A stable GPUI element key for a [`Uuid`]-identified row.
///
/// GPUI element ids take a `&'static str` or an integer, not a `Uuid`, so a
/// row's id has to be narrowed to 64 bits. Element ids only scope
/// interaction state (hover, press) within one parent, so the birthday-bound
/// collision risk across a handful of sibling rows is not a concern - what
/// matters is that the key follows the *entry* rather than its position,
/// which a list index does not.
pub(crate) fn element_key(id: Uuid) -> u64 {
    id.as_u64_pair().0
}

/// The font family to actually render the terminal with: the user's
/// `terminal_font_name` setting if GPUI can actually resolve it (checked
/// against the platform's font catalog plus whatever we've embedded),
/// otherwise the embedded JetBrains Mono default. Guards against a stale or
/// otherwise-unresolvable persisted value (an old default, a font that was
/// uninstalled, a font-panel value AppKit accepts but GPUI's lookup
/// doesn't) silently falling back further to the proportional UI font.
pub(crate) fn terminal_font_family(settings: &knot_core::Settings, cx: &App)
                                   -> gpui_kit::SharedString {
    let requested = &settings.terminal_font_name;
    if cx.text_system()
         .all_font_names()
         .iter()
         .any(|name| name == requested)
    {
        requested.clone().into()
    }
    else {
        "JetBrains Mono".into()
    }
}

/// Measures the actual rendered cell size for `terminal_view`'s font/size,
/// rather than guessing - an overestimate (e.g. a fixed 18px row height for
/// a font that actually renders taller) reports more PTY rows than fit in
/// the pane, so content the running program draws near what it thinks is
/// the bottom (an input box, a status line) ends up laid out below the
/// visible container and never appears.
pub(crate) fn terminal_cell_size(cx: &App, font_family: gpui_kit::SharedString,
                                 font_size: gpui_kit::Pixels)
                                 -> (f32, f32) {
    let font_id = cx.text_system().resolve_font(&gpui_kit::font(font_family));
    let width = cx.text_system()
                  .em_advance(font_id, font_size)
                  .unwrap_or(px(8.));
    let ascent = cx.text_system().ascent(font_id, font_size);
    let descent = cx.text_system().descent(font_id, font_size);
    (f32::from(width).max(1.), f32::from(ascent + descent).max(1.))
}

pub(crate) fn format_token_count(tokens: u64) -> String {
    let digits = tokens.to_string();
    let first_group = digits.len() % 3;
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.bytes().enumerate() {
        if index > 0 && index >= first_group && (index - first_group).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(char::from(digit));
    }

    formatted
}

pub(crate) fn context_usage_tooltip(used: u64, size: u64) -> String {
    format!("{}: {} / {} tokens",
            knot_core::l10n::t("panel.context_usage"),
            format_token_count(used),
            format_token_count(size),)
}

pub(crate) fn context_usage_indicator(used: u64, size: u64, cx: &App) -> impl IntoElement {
    let progress = (used as f64 / size as f64).clamp(0., 1.) as f32;
    let foreground = cx.theme().accent;
    let background = cx.theme().muted_foreground.opacity(0.35);
    let tooltip = context_usage_tooltip(used, size);

    div().id("panel-context-indicator")
         .size(px(16.))
         .flex_shrink_0()
         .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
         .child(canvas(move |bounds, _, _| {
                           let center = bounds.center();
                           let radius = (f32::from(bounds.size.width).min(f32::from(bounds.size
                                                                                          .height))
                                         / 2.
                                         - 1.)
                                              .max(0.);
                           let center_x = f32::from(center.x);
                           let center_y = f32::from(center.y);
                           let start = point(px(center_x), px(center_y - radius));
                           let radii = point(px(radius), px(radius));

                           let mut track = PathBuilder::stroke(px(2.));
                           track.move_to(start);
                           track.arc_to(radii,
                                        px(0.),
                                        false,
                                        true,
                                        point(px(center_x), px(center_y + radius)));
                           track.arc_to(radii, px(0.), false, true, start);

                           let progress_path = (progress > 0.).then(|| {
                                                   let mut path = PathBuilder::stroke(px(2.));
                                                   path.move_to(start);
                                                   if progress >= 1. {
                                                       path.arc_to(radii,
                                                                   px(0.),
                                                                   false,
                                                                   true,
                                                                   point(px(center_x),
                                                                         px(center_y + radius)));
                                                       path.arc_to(radii,
                                                                   px(0.),
                                                                   false,
                                                                   true,
                                                                   start);
                                                   }
                                                   else {
                                                       let angle = std::f32::consts::TAU * progress
                                                                   - std::f32::consts::FRAC_PI_2;
                                                       let end = point(px(center_x
                                                                          + radius * angle.cos()),
                                                                       px(center_y
                                                                          + radius * angle.sin()));
                                                       path.arc_to(radii,
                                                                   px(0.),
                                                                   progress > 0.5,
                                                                   true,
                                                                   end);
                                                   }
                                                   path.build().ok()
                                               })
                                               .flatten();

                           (track.build().ok(), progress_path)
                       },
                       move |_, (track, progress_path), window, _| {
                           if let Some(track) = track {
                               window.paint_path(track, background);
                           }
                           if let Some(progress_path) = progress_path {
                               window.paint_path(progress_path, foreground);
                           }
                       }).size(px(16.)))
}

#[cfg(test)]
mod context_usage_tests {
    use super::*;

    #[test]
    fn formats_token_counts_with_grouping() {
        assert_eq!(format_token_count(1_234_567), "1,234,567");
        assert_eq!(format_token_count(123), "123");
        assert_eq!(format_token_count(12), "12");
        assert_eq!(format_token_count(0), "0");
    }
}
