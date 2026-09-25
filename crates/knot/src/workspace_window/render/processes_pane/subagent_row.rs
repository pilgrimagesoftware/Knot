//! One subagent, as a row.
//!
//! Contract: `openspec/specs/agent-subagents/spec.md`, and
//! `openspec/specs/agent-processes/spec.md` - "The agent's pane presents a
//! processes section".
//!
//! Deliberately not a [`super::process_row`] with fields blanked out. A
//! subagent has no process identifier, cannot be signalled, and its elapsed
//! time comes from a record rather than from `ps`; a shared row that answered
//! all of those with `None` would be a row whose meaning depends on reading
//! its own emptiness.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use knot_subagents::{Subagent, SubagentState};

use super::chrome::{RUNTIME_WIDTH, STATE_WIDTH, row_action};
use super::text::runtime_text;
use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;

#[cfg(test)]
mod tests;

/// What a subagent row shows, resolved before any element is built.
///
/// `task` is the agent's own text, verbatim and untruncated - the row
/// truncates visually, and the copy action must hand over the whole thing.
/// Same bargain [`super::process_row::RowFields`] makes with a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SubagentFields {
    pub(super) kind:    String,
    pub(super) task:    String,
    pub(super) runtime: String,
    pub(super) state:   String,
}

/// The state's word, from the catalogue.
///
/// A failure's reason is *not* folded in here: it is the agent's own text and
/// belongs beside the task, not inside a label this file chose the shape of.
pub(super) fn state_text(state: &SubagentState) -> String {
    knot_core::l10n::t(match state {
                           SubagentState::Running => "processes.subagent_running",
                           SubagentState::Finished => "processes.subagent_finished",
                           SubagentState::Failed { .. } => "processes.subagent_failed",
                       })
}

pub(super) fn subagent_fields(subagent: &Subagent, now: std::time::Instant) -> SubagentFields {
    SubagentFields {
        // The kind is the agent's own data and is shown verbatim; the word
        // for *having no kind* is ours, so only that one is looked up.
        kind:    subagent.kind
                         .name()
                         .map_or_else(|| knot_core::l10n::t("processes.subagent_kind_unstated"),
                                      str::to_owned),
        task:    single_line(&subagent.task),
        runtime: runtime_text(subagent.elapsed(now)),
        state:   state_text(&subagent.state),
    }
}

impl WorkspaceWindow {
    /// One subagent: its kind, then its task on a single truncating line,
    /// then the runtime and state columns and the one action it offers.
    ///
    /// No process identifier column, because a subagent has none - the space
    /// the PID would occupy goes to the task instead.
    pub(super) fn subagent_row(&self, subagent: &Subagent, now: std::time::Instant,
                               cx: &mut Context<Self>)
                               -> gpui_kit::AnyElement {
        let fields = subagent_fields(subagent, now);
        let full_task = subagent.task.clone();
        let row_id = subagent.id.as_str().to_owned();
        let failure = subagent.state.failure_reason().map(str::to_owned);

        h_flex().w_full()
                .min_w_0()
                .items_center()
                .gap_3()
                .px_5()
                .py_1()
                .hover(|style| style.bg(cx.theme().muted))
                .child(div().flex_shrink_0()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child(fields.kind.clone()))
                // `min_w_0` on the growing child, not just `flex_1`: without
                // it the default `min-width: auto` lets a long task stretch
                // the row and push the columns off the pane.
                .child(div().flex_1()
                            .min_w_0()
                            .text_sm()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(match failure {
                                       Some(reason) => {
                                           knot_core::l10n::t_with("processes.\
                                                                    subagent_failed_reason",
                                                                   &[("reason", &reason)])
                                       }
                                       None => fields.task.clone(),
                                   }))
                .child(div().flex_shrink_0()
                            .w(px(RUNTIME_WIDTH))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(fields.runtime.clone()))
                .child(div().flex_shrink_0()
                            .w(px(STATE_WIDTH))
                            .text_xs()
                            .text_color(if subagent.state.failure_reason().is_some() {
                                            cx.theme().danger
                                        }
                                        else {
                                            cx.theme().muted_foreground
                                        })
                            .child(fields.state.clone()))
                .child(self.subagent_row_actions(&row_id, full_task, cx))
                .into_any_element()
    }

    /// The row's one trailing control.
    ///
    /// No terminate: a subagent has no process identifier to signal, and the
    /// only way to stop one is to interrupt the parent agent, which is a
    /// different action with a different consequence. No copy-identifier and
    /// no process viewer either - neither has anything to refer to.
    fn subagent_row_actions(&self, row_id: &str, task: String, cx: &mut Context<Self>)
                            -> gpui_kit::AnyElement {
        let muted = cx.theme().muted_foreground;

        h_flex().flex_shrink_0()
                .items_center()
                .gap_1()
                .child(row_action(("copy-subagent-task", stable_row_id(row_id)),
                                  IconName::Copy,
                                  "processes.copy_task",
                                  muted,
                                  false,
                                  cx.listener(move |view, _: &ClickEvent, window, cx| {
                                        view.copy_subagent_task(task.clone(), window, cx);
                                    })))
                .into_any_element()
    }
}

/// A stable element id for a subagent, whose own identifier is a string the
/// agent chose while [`row_action`] keys on a number.
///
/// Only has to be stable within one render of one agent's list, which a hash
/// of the identifier is. A collision would merge two rows' click targets, so
/// it hashes the whole identifier rather than truncating it.
fn stable_row_id(id: &str) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}
