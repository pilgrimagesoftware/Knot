//! Tool-call cards: the header that names the call and its status, the body
//! that shows what it produced, and the diff view a file edit gets.
//!
//! Status arrives as an ACP string rather than an enum, so every lookup here
//! falls through to something safe for a value Knot has not seen - an
//! unrecognized status renders as its own text rather than vanishing.

use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::rc::Rc;

use gpui_kit::ClickEvent;
use gpui_kit::Hsla;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use gpui_kit::rgb;

use crate::app_support::single_line;
use crate::panel_state::ToolCallCard;
use crate::panel_view::style::CardOutline;
use crate::panel_view::style::ERROR_COLOR;
use crate::panel_view::style::MUTED;
use crate::panel_view::style::PanelStyle;
use crate::panel_view::style::SAFE_COLOR;
use crate::panel_view::style::card_outline;

/// A tool-call card: an icon/title/status header over whatever content
/// the agent has reported so far - diff blocks as an added/removed line
/// view, everything else as monospace output (`render_tool_call_body`).
/// The header separates content from chrome by font: the title is a
/// command, a path or an identifier and renders monospace, the status is
/// the panel's own word for the call and stays proportional. The outline
/// is coloured by `card_outline`.
///
/// `collapsed` comes from `PanelState::is_collapsed`, so a succeeded call
/// folds to just this header. The whole header row is the toggle's hit
/// target, not only the chevron: a 12px icon is a poor one and the header
/// carries no other action (design decision "The control is a disclosure
/// chevron, and the whole header toggles").
pub(super) fn render_tool_call_card(card: &ToolCallCard, style: &PanelStyle, collapsed: bool,
                                    on_toggle: Rc<dyn Fn(String)>)
                                    -> impl IntoElement {
    let label = if card.title.is_empty() {
        card.kind.clone()
    }
    else {
        card.title.clone()
    };
    let id = card.id.clone();
    v_flex().w_full()
            .min_w_0()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(style.outline_color(card_outline(&card.status)))
            .bg(style.card_color)
            .child(h_flex().id(("panel-tool-call-header", element_id(&card.id)))
                           .w_full()
                           .min_w_0()
                           .gap_2()
                           .items_center()
                           .cursor_pointer()
                           .on_click(move |_: &ClickEvent, _, _| on_toggle(id.clone()))
                           .child(Icon::new(disclosure_icon(collapsed)).xsmall()
                                                                       .text_color(rgb(MUTED)))
                           .child(Icon::new(tool_call_icon(&card.kind)).xsmall()
                                                                       .text_color(rgb(MUTED)))
                           .child({
                               let title = div().flex_1()
                                                .min_w_0()
                                                .font_family(style.mono_font_family.clone())
                                                .text_xs()
                                                .text_color(rgb(MUTED));
                               if collapsed {
                                   // Joined before it is handed over: the
                                   // style flags below only stop *soft*
                                   // wrapping, so a title carrying line
                                   // breaks - a heredoc command, say - would
                                   // otherwise draw one line per break and
                                   // fill the pane. Expanded keeps `label`
                                   // verbatim.
                                   title.overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(single_line(&label))
                               }
                               else {
                                   title.child(label)
                               }
                           })
                           .children(status_icon(&card.status).map(|icon| {
                                         Button::new(("tool-call-status", element_id(&card.id)))
                        .icon(icon)
                        .ghost()
                        .tooltip(status_label(&card.status))
                        .text_color(status_icon_color(&card.status, style))
                                     }))
                           .children(status_icon(&card.status).is_none().then(|| {
                                                                            div().flex_shrink_0()
                                    .font_family(style.ui_font_family.clone())
                                    .text_xs()
                                    .child(status_label(&card.status))
                                                                        })))
            .children((!collapsed).then(|| render_tool_call_body(card, style)))
}

/// A card's content, drawn only while the card is expanded. An unfinished
/// call with no content yet shows an in-progress placeholder; a *finished*
/// one with no content shows nothing rather than a stale "Running…".
pub(super) fn render_tool_call_body(card: &ToolCallCard, style: &PanelStyle) -> impl IntoElement {
    v_flex()
        .w_full()
        .min_w_0()
        .gap_2()
        .children(
            card.content
                .iter()
                .map(|content| render_tool_call_content(content, style)),
        )
        .children((card.content.is_empty() && !card.is_finished()).then(in_progress_placeholder))
}

/// The disclosure chevron for a card in either state: pointing right at a
/// collapsed card (its content is off to the side, unopened) and down at
/// an expanded one (its content is below), the platform convention.
pub(super) fn disclosure_icon(collapsed: bool) -> IconName {
    if collapsed {
        IconName::ChevronRight
    }
    else {
        IconName::ChevronDown
    }
}

/// A stable element id for a tool call's header. GPUI element ids are
/// `&'static str` or an integer, and a tool-call id is neither, so it is
/// hashed - collisions only cost the wrong card's click state, and within
/// one conversation they are not realistic.
pub(super) fn element_id(tool_call_id: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    tool_call_id.hash(&mut hasher);
    hasher.finish()
}

/// One tool-call content block. Diffs get the added/removed line view
/// `acp-panel-ui`'s tool-call-rendering requirement asks for; text and
/// terminal references fall back to monospace output.
pub(super) fn render_tool_call_content(content: &knot_acp::ToolCallContent, style: &PanelStyle)
                                       -> gpui_kit::AnyElement {
    match content {
        knot_acp::ToolCallContent::Diff { path,
                                          old_text,
                                          new_text, } => {
            render_diff(path, old_text.as_deref(), new_text, style).into_any_element()
        }
        knot_acp::ToolCallContent::Text(text) => render_output_text(text, style).into_any_element(),
        knot_acp::ToolCallContent::Terminal { terminal_id } => {
            render_output_text(&format!("[terminal {terminal_id}]"), style).into_any_element()
        }
    }
}

/// Human-readable form of an ACP tool-call status. Unrecognized statuses
/// pass through unchanged rather than being swallowed - `status` is a
/// plain wire string, and a future value is more useful shown than hidden.
pub(super) fn status_label(status: &str) -> String {
    match status {
        "pending" => "Pending".to_string(),
        "in_progress" => "Running…".to_string(),
        "completed" => "Done".to_string(),
        "failed" => "Failed".to_string(),
        other => other.to_string(),
    }
}

pub(super) fn status_icon(status: &str) -> Option<IconName> {
    match status {
        "pending" | "in_progress" => Some(IconName::Loader),
        "completed" => Some(IconName::CircleCheck),
        "failed" => Some(IconName::CircleX),
        _ => None,
    }
}

pub(super) fn status_icon_color(status: &str, style: &PanelStyle) -> Hsla {
    match card_outline(status) {
        CardOutline::Neutral => rgb(MUTED).into(),
        outline => style.outline_color(outline),
    }
}

/// Maps an ACP tool-call `kind` to an identifying icon, per the response
/// action bar design's "icon lookup keyed on kind" decision. `kind` is a
/// plain string off the wire rather than a closed enum, so the match is
/// string-keyed with a generic fallback arm rather than truly exhaustive.
pub(super) fn tool_call_icon(kind: &str) -> IconName {
    match kind {
        "read" => IconName::FileText,
        "edit" => IconName::Pencil,
        "delete" => IconName::Trash,
        "move" => IconName::Move,
        "search" => IconName::Search,
        "execute" => IconName::Terminal,
        "think" => IconName::Brain,
        "fetch" => IconName::Globe,
        _ => IconName::Wrench,
    }
}

pub(super) fn in_progress_placeholder() -> impl IntoElement {
    div().text_xs()
         .text_color(rgb(MUTED))
         .child(knot_core::l10n::t("panel.running"))
}

/// A file-edit diff as an added/removed line view rather than raw text,
/// per `acp-panel-ui`'s tool-call-rendering requirement. `old_text` is
/// `None` for a newly created file, in which case every line is an
/// addition.
pub(super) fn render_diff(path: &str, old_text: Option<&str>, new_text: &str, style: &PanelStyle)
                          -> impl IntoElement {
    v_flex().w_full()
            .min_w_0()
            .gap_1()
            .child(div().w_full()
                        .min_w_0()
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(path.to_string()))
            .child(v_flex().w_full()
                           .min_w_0()
                           .children(diff_lines(old_text, new_text).into_iter()
                                                                   .map(|(color, text)| {
                                                                       div().w_full()
                                               .min_w_0()
                                               .font_family(style.mono_font_family.clone())
                                               .text_xs()
                                               .text_color(rgb(color))
                                               .child(text)
                                                                   })))
}

/// The `(color, text)` line list for a diff. With an `old_text` this is a
/// whole-file replacement, so every old line reads as removed and every
/// new line as added; without one the `new_text` is already in unified
/// form (or is a brand-new file) and its own `+`/`-` prefixes decide.
pub(super) fn diff_lines(old_text: Option<&str>, new_text: &str) -> Vec<(u32, String)> {
    match old_text {
        Some(old) => old.lines()
                        .map(|line| (ERROR_COLOR, format!("- {line}")))
                        .chain(new_text.lines()
                                       .map(|line| (SAFE_COLOR, format!("+ {line}"))))
                        .collect(),
        None => new_text.lines()
                        .map(|line| match line.strip_prefix('+') {
                            Some(added) => (SAFE_COLOR, format!("+ {added}")),
                            None => match line.strip_prefix('-') {
                                Some(removed) => (ERROR_COLOR, format!("- {removed}")),
                                None => (MUTED, line.to_string()),
                            },
                        })
                        .collect(),
    }
}

/// Monospace tool output. `w_full`/`min_w_0` so a long line wraps inside
/// the card instead of stretching the whole conversation pane, per
/// `knot-ui-conventions.md`'s "Flex overflow" rule.
pub(super) fn render_output_text(text: &str, style: &PanelStyle) -> impl IntoElement {
    div().w_full()
         .min_w_0()
         .font_family(style.mono_font_family.clone())
         .text_xs()
         .text_color(rgb(MUTED))
         .child(text.to_string())
}
