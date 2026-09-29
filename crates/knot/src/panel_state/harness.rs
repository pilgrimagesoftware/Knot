//! Telling a replayed user chunk the user typed from one the agent's harness
//! injected (`openspec/specs/acp-panel-ui`, "Harness-injected messages are
//! not shown as the user's").
//!
//! Claude Code stores what its harness adds to a conversation - a
//! `<task-notification>` when a background task finishes, a
//! `<system-reminder>` it appends for the model - as `type: "user"` messages.
//! Live, `claude-agent-acp` drops them before they reach Knot; on
//! `session/load` it replays every stored user message as a
//! `user_message_chunk`, so after a resume they rendered as prompt bubbles,
//! raw XML and all (#551).
//!
//! Pure, so both ways of recognising one are tested without a session.

use serde_json::Value;

/// Where a replayed user chunk belongs in the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum UserChunk {
    /// The user's own words: part of a prompt bubble.
    Human,
    /// Harness text meant for the model alone - `<system-reminder>` blocks -
    /// which the live stream never shows either.
    Hidden,
    /// A harness event worth a compact row, with the row's text.
    Notice(String),
}

/// The `_claude/origin` kinds a person produced. Everything else is the
/// harness, matching `claude-agent-acp`'s `AUTONOMOUS_RESULT_ORIGINS`.
const HUMAN_ORIGINS: [&str; 2] = ["human", "channel"];

/// The blocks that make a chunk the harness's rather than the user's.
const TASK_NOTIFICATION: &str = "task-notification";
const SYSTEM_REMINDER: &str = "system-reminder";

/// Classifies one replayed user chunk.
///
/// An origin tag in `meta` wins when there is one: it is what the adapter
/// knows, not a guess from the text. Without one, a chunk is the harness's
/// only if it consists *entirely* of `<task-notification>` and
/// `<system-reminder>` blocks - a prompt that merely mentions a tag, or
/// quotes one inside other words, is still the user's.
pub(super) fn classify_user_chunk(text: &str, meta: Option<&Value>) -> UserChunk {
    let injected = match origin_kind(meta) {
        Some(kind) => !HUMAN_ORIGINS.contains(&kind),
        None => harness_blocks(text).is_some(),
    };
    if !injected {
        return UserChunk::Human;
    }
    let blocks = harness_blocks(text).unwrap_or_default();
    if let Some(notification) = blocks.iter().find(|block| block.tag == TASK_NOTIFICATION) {
        return UserChunk::Notice(task_finished(notification.body));
    }
    if !blocks.is_empty() {
        return UserChunk::Hidden;
    }
    // Tagged as injected, but not in a shape this recognises. Say that it
    // happened rather than dropping it, and rather than showing text written
    // for the model.
    UserChunk::Notice(knot_core::l10n::t("panel.harness.automated"))
}

/// `meta["_claude/origin"].kind`, when an adapter sends it.
fn origin_kind(meta: Option<&Value>) -> Option<&str> {
    meta?.get("_claude/origin")?.get("kind")?.as_str()
}

/// One harness block: its tag and what is between its open and close tags.
#[derive(Debug)]
struct Block<'a> {
    tag:  &'a str,
    body: &'a str,
}

/// `text` split into harness blocks, or `None` if anything but whitespace
/// lies outside them - including text with no blocks at all.
fn harness_blocks(text: &str) -> Option<Vec<Block<'_>>> {
    let mut blocks = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        let tag = [TASK_NOTIFICATION, SYSTEM_REMINDER].into_iter()
                                                      .find(|tag| opens(rest, tag))?;
        let open_end = rest.find('>')? + 1;
        let close = format!("</{tag}>");
        let body_len = rest[open_end..].find(&close)?;
        blocks.push(Block { tag,
                            body: &rest[open_end..open_end + body_len] });
        rest = rest[open_end + body_len + close.len()..].trim_start();
    }
    (!blocks.is_empty()).then_some(blocks)
}

/// Whether `text` starts with `tag`'s opening tag, attributes allowed.
fn opens(text: &str, tag: &str) -> bool {
    text.strip_prefix('<')
        .and_then(|rest| rest.strip_prefix(tag))
        .and_then(|rest| rest.chars().next())
        .is_some_and(|next| next == '>' || next.is_whitespace())
}

/// The row for a finished background task: its `<summary>` when it has one.
fn task_finished(body: &str) -> String {
    let summary = body.find("<summary>")
                      .map(|start| &body[start + "<summary>".len()..])
                      .and_then(|rest| rest.find("</summary>").map(|end| &rest[..end]))
                      .map(|summary| summary.split_whitespace().collect::<Vec<_>>().join(" "))
                      .filter(|summary| !summary.is_empty());
    match summary {
        Some(summary) => {
            knot_core::l10n::t_with("panel.harness.task_finished", &[("summary", &summary)])
        }
        None => knot_core::l10n::t("panel.harness.task_finished_unnamed"),
    }
}

#[cfg(test)]
mod tests;
