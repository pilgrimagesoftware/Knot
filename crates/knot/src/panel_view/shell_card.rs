//! The card for a `!` command the user ran: what ran, where, what it
//! produced, and whether the agent has been told.
//!
//! Contract: `openspec/specs/panel-shell-passthrough/spec.md`.
//!
//! It borrows the tool-call card's shape -- a header over monospace output on
//! the raised card surface -- and departs from it in the one way that matters:
//! the header leads with a `$` prompt glyph and the command itself, so a
//! reader can tell at a glance that a person ran this and not the agent. That
//! is the whole of `acp-panel-ui`'s "a shell result reads as neither message
//! nor tool call".

use std::rc::Rc;

use gpui_kit::ClickEvent;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use gpui_kit::rgb;
use knot_processes::ShellStatus;
use knot_processes::consts::SHELL_TIMEOUT;

use crate::panel_state::ShellCard;
use crate::panel_view::style::ERROR_COLOR;
use crate::panel_view::style::MUTED;
use crate::panel_view::style::PanelStyle;

/// The prompt glyph the header leads with. Not localized: it is the shell's
/// own mark, the same in every language, and translating it would make the
/// card stop looking like a terminal line.
const PROMPT_GLYPH: &str = "$";

/// A stable element id for one card's controls.
///
/// `ElementId` takes a `(&str, u64)` but not a `(&str, Uuid)`, and the high
/// half of a v4 uuid is as distinguishing as the low half, so either would do.
fn element_id(id: uuid::Uuid) -> u64 {
    id.as_u64_pair().0
}

pub(super) fn render_shell_card(card: &ShellCard, style: &PanelStyle,
                                on_cancel: Rc<dyn Fn(uuid::Uuid)>,
                                on_discard: Rc<dyn Fn(uuid::Uuid)>)
                                -> impl IntoElement {
    let running = card.is_running();
    let failed = card.status().is_failure();

    v_flex().w_full()
            .min_w_0()
            .gap_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(if failed {
                              style.danger_color
                          }
                          else if running {
                              style.info_color
                          }
                          else {
                              style.border_color
                          })
            .bg(style.card_color)
            .child(render_header(card, style, running, on_cancel))
            .children(render_output(card, style))
            .children(render_footer(card, style, on_discard))
}

/// `$ command` on the left, where it ran on the right.
fn render_header(card: &ShellCard, style: &PanelStyle, running: bool,
                 on_cancel: Rc<dyn Fn(uuid::Uuid)>)
                 -> impl IntoElement {
    let id = card.id;

    h_flex().w_full()
            .min_w_0()
            .gap_2()
            .items_start()
            .child(div().font_family(style.mono_font_family.clone())
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(PROMPT_GLYPH))
            .child(div().flex_1()
                        .min_w_0()
                        .font_family(style.mono_font_family.clone())
                        .text_xs()
                        .child(card.command.clone()))
            .child(div().font_family(style.ui_font_family.clone())
                        .text_xs()
                        .text_color(rgb(MUTED))
                        .child(knot_core::l10n::t_with("panel.shell.ran_in",
                                                       &[("folder", &card.cwd)])))
            .children(running.then(|| {
                                 Button::new(("panel-shell-cancel", element_id(id)))
                .icon(IconName::Close)
                .tooltip(knot_core::l10n::t("panel.shell.cancel"))
                .xsmall()
                .ghost()
                .on_click(move |_: &ClickEvent, _, _| on_cancel(id))
                             }))
}

/// Both streams, in one monospace block, with stderr in the error colour so
/// the reader keeps which stream said what.
fn render_output(card: &ShellCard, style: &PanelStyle) -> Vec<gpui_kit::AnyElement> {
    let mut blocks: Vec<gpui_kit::AnyElement> = Vec::new();

    if !card.run.stdout.is_empty() {
        blocks.push(div().w_full()
                         .min_w_0()
                         .font_family(style.mono_font_family.clone())
                         .text_xs()
                         .child(card.run.stdout.text().to_owned())
                         .into_any_element());
    }

    if !card.run.stderr.is_empty() {
        blocks.push(div().w_full()
                         .min_w_0()
                         .font_family(style.mono_font_family.clone())
                         .text_xs()
                         .text_color(rgb(ERROR_COLOR))
                         .child(card.run.stderr.text().to_owned())
                         .into_any_element());
    }

    if card.run.is_truncated() {
        blocks.push(note(style, knot_core::l10n::t("panel.shell.truncated")).into_any_element());
    }

    blocks
}

/// The status line, and the discard control when a result is waiting.
fn render_footer(card: &ShellCard, style: &PanelStyle, on_discard: Rc<dyn Fn(uuid::Uuid)>)
                 -> Vec<gpui_kit::AnyElement> {
    let id = card.id;
    let mut rows: Vec<gpui_kit::AnyElement> = Vec::new();

    rows.push(h_flex().w_full()
                      .min_w_0()
                      .gap_2()
                      .items_center()
                      .child(note(style, status_label(card.status())))
                      .children(delivery_label(card).map(|label| note(style, label)))
                      .children(card.is_pending().then(|| {
                                    Button::new(("panel-shell-discard", element_id(id)))
                    .label(knot_core::l10n::t("panel.shell.discard"))
                    .xsmall()
                    .ghost()
                    .on_click(move |_: &ClickEvent, _, _| on_discard(id))
                                }))
                      .into_any_element());

    rows
}

/// The panel's own words about the run, so proportional rather than
/// monospace - the same split the tool-call header makes between a command
/// and a status.
fn note(style: &PanelStyle, text: String) -> impl IntoElement {
    div().font_family(style.ui_font_family.clone())
         .text_xs()
         .text_color(rgb(MUTED))
         .child(text)
}

/// What the card says the run came to.
pub(super) fn status_label(status: &ShellStatus) -> String {
    match status {
        ShellStatus::Running => knot_core::l10n::t("panel.shell.running"),
        ShellStatus::Exited { code: 0 } => knot_core::l10n::t("panel.shell.exit_ok"),
        ShellStatus::Exited { code } => {
            knot_core::l10n::t_with("panel.shell.exit_failed", &[("code", &code.to_string())])
        }
        ShellStatus::Signalled => knot_core::l10n::t("panel.shell.signalled"),
        ShellStatus::Cancelled => knot_core::l10n::t("panel.shell.cancelled"),
        ShellStatus::TimedOut => {
            knot_core::l10n::t_with("panel.shell.timed_out", &[("limit", &timeout_label())])
        }
        ShellStatus::FailedToStart { message } => {
            knot_core::l10n::t_with("panel.shell.failed_to_start", &[("error", message)])
        }
    }
}

/// The wall-clock limit, in the largest unit it divides evenly into.
///
/// Whole minutes where it fits, because "took longer than 2 minutes" is what
/// the reader can act on and "took longer than 120 seconds" is the same fact
/// worse said.
pub(super) fn timeout_label() -> String {
    let seconds = SHELL_TIMEOUT.as_secs();

    if seconds >= 60 && seconds.is_multiple_of(60) {
        let minutes = seconds / 60;
        return format!("{minutes} {}",
                       knot_core::l10n::pluralize(minutes, "count.minute", "count.minutes"));
    }

    format!("{seconds} {}",
            knot_core::l10n::pluralize(seconds, "count.second", "count.seconds"))
}

/// Whether the agent has been told, when there is anything to tell.
pub(super) fn delivery_label(card: &ShellCard) -> Option<String> {
    use crate::panel_state::ShellDelivery;

    match card.delivery {
        ShellDelivery::None => None,
        ShellDelivery::Pending => Some(knot_core::l10n::t("panel.shell.pending")),
        ShellDelivery::Shared => Some(knot_core::l10n::t("panel.shell.shared")),
    }
}
