//! Derives the compact tool-call summary from the message list.
//!
//! Owns the "what did this run of tool calls do" question and nothing
//! else: it reads [`super::PanelState::messages`] and never mutates it, so
//! turning compact mode on or off changes only what is drawn, never what
//! is stored. That is what lets a reader switch modes mid-conversation
//! without losing a single tool result.
//!
//! Contract: `openspec/specs/collapsed-tool-call-summary/spec.md`.

use std::collections::BTreeSet;

use knot_acp::ToolCallContent;

use super::{PanelMessage, PanelState, ToolCallCard};

/// What one run of contiguous tool calls did, as the summary line reports
/// it. Every count is best-effort: an adapter that declares no `kind` for
/// its calls still yields `calls`, and the secondary counts stay zero
/// rather than being guessed at from display titles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ToolRunSummary {
    /// Tool calls in the run. Always known.
    pub calls:        usize,
    pub files_edited: usize,
    pub files_read:   usize,
    pub commands_run: usize,
    /// Calls that reported `failed`. Non-zero keeps the failure visible in
    /// compact mode, per the spec's "Failed call remains represented".
    pub failed:       usize,
    /// Whether any call in the run has yet to finish, which is what makes
    /// the line read as live rather than final.
    pub running:      bool,
}

/// The ACP tool kinds the summary counts. Everything else (`think`,
/// `fetch`, `search`, `other`, or an adapter's own) still counts toward
/// `calls` and contributes no secondary count.
const KIND_EDIT: &str = "edit";
const KIND_READ: &str = "read";
const KIND_EXECUTE: &str = "execute";

impl ToolRunSummary {
    /// Folds one card into the summary. `edited_paths` collects the
    /// distinct files named by diff blocks, so a file edited twice in one
    /// run counts once.
    fn absorb(&mut self, card: &ToolCallCard, edited_paths: &mut BTreeSet<String>) {
        self.calls += 1;
        if card.status == "failed" {
            self.failed += 1;
        }
        if !card.is_finished() {
            self.running = true;
        }
        match card.kind.as_str() {
            KIND_EDIT => {
                let paths = diff_paths(card);
                if paths.is_empty() {
                    // An edit whose diff has not arrived (or that the
                    // adapter never sends one for) still edited a file.
                    self.files_edited += 1;
                }
                else {
                    edited_paths.extend(paths);
                }
            }
            KIND_READ => self.files_read += 1,
            KIND_EXECUTE => self.commands_run += 1,
            _ => {}
        }
    }
}

/// The distinct files a call's diff blocks name.
fn diff_paths(card: &ToolCallCard) -> Vec<String> {
    card.content
        .iter()
        .filter_map(|content| match content {
            ToolCallContent::Diff { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect()
}

impl PanelState {
    /// Whether `index` is the first tool call of a run - the message that
    /// carries the whole run's summary line in compact mode. A tool call
    /// whose predecessor is also a tool call continues a run and draws
    /// nothing of its own.
    ///
    /// Any non-tool message is a hard boundary, per the spec's "Summary
    /// boundaries are explicit": assistant text, a new user prompt, or an
    /// error all end the run, and the next tool call starts a fresh one.
    pub fn starts_tool_run(&self, index: usize) -> bool {
        if !matches!(self.messages.get(index), Some(PanelMessage::ToolCall(_))) {
            return false;
        }
        let previous = index.checked_sub(1)
                            .and_then(|prev| self.messages.get(prev));
        !matches!(previous, Some(PanelMessage::ToolCall(_)))
    }

    /// Whether `index` is a tool call that a run already covers, and so
    /// draws nothing in compact mode.
    pub fn continues_tool_run(&self, index: usize) -> bool {
        matches!(self.messages.get(index), Some(PanelMessage::ToolCall(_)))
        && !self.starts_tool_run(index)
    }

    /// Summarizes the run starting at `index`, reading forward to the
    /// first non-tool message. Returns the zero summary when `index` is
    /// not a tool call.
    ///
    /// Linear in the run's length, not the conversation's, and only the
    /// visible rows are ever asked - the virtualizer draws a handful at a
    /// time.
    pub fn tool_run_summary(&self, index: usize) -> ToolRunSummary {
        let mut summary = ToolRunSummary::default();
        let mut edited_paths = BTreeSet::new();
        for message in &self.messages[index.min(self.messages.len())..] {
            let PanelMessage::ToolCall(card) = message
            else {
                break;
            };
            summary.absorb(card, &mut edited_paths);
        }
        summary.files_edited += edited_paths.len();
        summary
    }

    /// The id of the first tool call of the run containing `index` - the
    /// key a run's open/closed choice is stored under. `None` when
    /// `index` is not a tool call.
    ///
    /// Walks back to the run's start, so it is linear in the run's length
    /// rather than the conversation's.
    pub fn tool_run_head(&self, index: usize) -> Option<&str> {
        if !matches!(self.messages.get(index), Some(PanelMessage::ToolCall(_))) {
            return None;
        }
        let start =
            (0..=index).rev()
                       .take_while(|i| {
                           matches!(self.messages.get(*i), Some(PanelMessage::ToolCall(_)))
                       })
                       .last()?;
        match self.messages.get(start) {
            Some(PanelMessage::ToolCall(card)) => Some(card.id.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
