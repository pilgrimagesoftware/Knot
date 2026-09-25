//! An agent panel's queue of prompts waiting for their turn.
//!
//! Every entry carries a stable identity, so each mutation names the entry
//! it means instead of a position. Deletion, a delivery result arriving off
//! the runtime, and the pump promoting the head all reach the queue
//! independently, and any one of them shifts every index behind it. Text is
//! no better as an identity: two prompts that read the same are ordinary,
//! and matching on text picks whichever comes first.
//!
//! Contract: `openspec/specs/queued-message-management/spec.md`.

use uuid::Uuid;

/// What put a prompt in the queue.
///
/// Carried explicitly rather than recovered from the prompt text, per
/// `mcp-messaging`'s "Inbox nudges preserve interrupted session work": an
/// automatic nudge and a user prompt that reads the same are
/// indistinguishable after the fact, so matching on the nudge's wording
/// would reclassify a user who pasted it.
///
/// Never serialized - the queue lives only as long as the window - so
/// unlike the vocabularies in `.claude/rules/rust-structure.md` this one
/// needs no `Display`/`FromStr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PromptOrigin {
    /// Typed by the user, or sent on the user's behalf by a broadcast.
    #[default]
    User,
    /// The automatic "check your inbox" nudge Knot sends of its own accord.
    InboxNudge,
    /// The agent's startup prompt, queued behind its registration turn on a
    /// fresh session. See `openspec/specs/agent-launch-command/spec.md`.
    Startup,
}

/// One prompt waiting to be delivered to an agent.
///
/// An entry leaves the queue the moment the pump hands it to the agent -
/// the prompt is in the conversation from then on, and a row left behind
/// for the length of the turn read as one still waiting. `failed` marks
/// one whose delivery returned an error; [`return_failed`] puts it back at
/// the head, where it holds its position until the user retries or deletes
/// it, rather than being retried automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedPanelPrompt {
    pub(crate) id:     Uuid,
    pub(crate) text:   String,
    pub(crate) failed: bool,
    /// What put this prompt here. Set at the one point where that is still
    /// known; nothing downstream can recover it.
    pub(crate) origin: PromptOrigin,
}

impl QueuedPanelPrompt {
    pub(crate) fn new(text: String, origin: PromptOrigin) -> Self {
        Self { id: Uuid::new_v4(),
               text,
               failed: false,
               origin }
    }
}

/// Removes the entry `id` names, leaving the rest of the queue in order.
///
/// Returns whether an entry was removed. An `id` that is already gone - a
/// delivery that landed between the click and this call - is not an error.
pub(crate) fn remove(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid) -> bool {
    let Some(index) = position(queue, id)
    else {
        return false;
    };
    queue.remove(index);
    true
}

/// Removes the entry `id` names and returns its text, for editing.
///
/// `None` means the entry is gone - already handed to the agent, which
/// cannot be taken back - and the caller leaves the composer as it found
/// it. The entry does not hold its place - edited text is sent as a new
/// prompt, at the back.
pub(crate) fn take(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid) -> Option<String> {
    let index = position(queue, id)?;
    Some(queue.remove(index).text)
}

/// Clears the failed mark on `id` so the pump offers the entry again.
///
/// The prompt keeps its place and its identity; retry re-delivers the same
/// message rather than enqueuing a new one.
pub(crate) fn retry(queue: &mut [QueuedPanelPrompt], id: Uuid) -> bool {
    let Some(prompt) = queue.iter_mut().find(|prompt| prompt.id == id)
    else {
        return false;
    };
    if !prompt.failed {
        return false;
    }
    prompt.failed = false;
    true
}

/// Takes the head of the queue for delivery.
///
/// A failed head is not taken: it waits for the user to retry or delete it,
/// and everything behind it waits too, so prompts are never delivered out
/// of the order they were queued in.
pub(crate) fn take_next(queue: &mut Vec<QueuedPanelPrompt>) -> Option<QueuedPanelPrompt> {
    if queue.first()?.failed {
        return None;
    }
    Some(queue.remove(0))
}

/// Puts a prompt whose delivery failed back at the head of the queue,
/// marked failed, with its text and identity intact.
///
/// The head, because it was the head when it was taken: the pump takes
/// nothing else while a delivery is outstanding, so everything still queued
/// was behind it.
pub(crate) fn return_failed(queue: &mut Vec<QueuedPanelPrompt>, mut prompt: QueuedPanelPrompt) {
    prompt.failed = true;
    queue.insert(0, prompt);
}

fn position(queue: &[QueuedPanelPrompt], id: Uuid) -> Option<usize> {
    queue.iter().position(|prompt| prompt.id == id)
}

/// Whether loading a queued message into the composer would destroy work,
/// and so has to be confirmed first.
///
/// Whitespace alone is not work: a stray newline in an untouched composer
/// should not make every edit ask.
pub(crate) fn needs_replace_confirmation(composer: &str) -> bool {
    !composer.trim().is_empty()
}

/// The localized name of a queued entry's state, for the row's status icon.
///
/// The icon carries the state visually and the row's only text is the
/// prompt itself, so this is the sole thing that tells a screen reader
/// whether the entry is waiting or stuck.
///
/// A queued inbox nudge says so. Since nudges queue behind a running turn
/// the user sees a prompt in their own queue that they never typed, and the
/// row's text - the nudge's wording - is the only other clue.
///
/// `failed` wins over the origin: a stuck entry is the state the user has
/// to act on, and where it came from does not change that.
pub(crate) fn queued_status_label(failed: bool, origin: PromptOrigin) -> String {
    knot_core::l10n::t(match (failed, origin) {
                           (true, _) => "panel.failed",
                           (false, PromptOrigin::InboxNudge) => "panel.queued_inbox_nudge",
                           (false, PromptOrigin::Startup) => "panel.queued_startup_prompt",
                           (false, PromptOrigin::User) => "panel.queued",
                       })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(texts: &[&str]) -> Vec<QueuedPanelPrompt> {
        texts.iter()
             .map(|text| QueuedPanelPrompt::new((*text).to_string(), PromptOrigin::User))
             .collect()
    }

    fn texts(queue: &[QueuedPanelPrompt]) -> Vec<&str> {
        queue.iter().map(|prompt| prompt.text.as_str()).collect()
    }

    /// A nudge that arrives mid-turn goes behind the work already waiting
    /// and stays marked as a nudge: nothing downstream can tell one from a
    /// user's prompt by its text, which is the whole point of the field.
    #[test]
    fn a_queued_nudge_waits_its_turn_and_keeps_its_origin() {
        let mut queue = queue(&["first", "second"]);
        queue.push(QueuedPanelPrompt::new("nudge".to_string(), PromptOrigin::InboxNudge));

        assert_eq!(texts(&queue), ["first", "second", "nudge"]);
        assert_eq!(queue.iter().map(|prompt| prompt.origin).collect::<Vec<_>>(),
                   [PromptOrigin::User,
                    PromptOrigin::User,
                    PromptOrigin::InboxNudge]);
    }

    /// Taking the entry in front of a nudge leaves the nudge where it is -
    /// the turn it was waiting behind ending is what promotes it, not
    /// anything about the nudge itself.
    #[test]
    fn taking_the_entry_ahead_promotes_the_nudge_unchanged() {
        let mut queue = queue(&["first"]);
        queue.push(QueuedPanelPrompt::new("nudge".to_string(), PromptOrigin::InboxNudge));

        assert_eq!(take_next(&mut queue).map(|prompt| prompt.text).as_deref(),
                   Some("first"));

        assert_eq!(texts(&queue), ["nudge"]);
        assert_eq!(queue[0].origin, PromptOrigin::InboxNudge);
    }

    #[test]
    fn removing_one_entry_keeps_the_order_of_the_rest() {
        let mut queue = queue(&["first", "second", "third"]);
        let second = queue[1].id;

        assert!(remove(&mut queue, second));

        assert_eq!(texts(&queue), ["first", "third"]);
    }

    #[test]
    fn removing_picks_the_named_entry_among_identical_text() {
        let mut queue = queue(&["same", "same", "same"]);
        let middle = queue[1].id;
        let (first, last) = (queue[0].id, queue[2].id);

        assert!(remove(&mut queue, middle));

        assert_eq!(queue.iter().map(|prompt| prompt.id).collect::<Vec<_>>(),
                   [first, last]);
    }

    #[test]
    fn removing_an_absent_id_changes_nothing() {
        let mut queue = queue(&["first"]);

        assert!(!remove(&mut queue, Uuid::new_v4()));

        assert_eq!(texts(&queue), ["first"]);
    }

    #[test]
    fn a_failed_entry_is_deletable() {
        let mut queue = queue(&["failed"]);
        queue[0].failed = true;
        let failed = queue[0].id;

        assert!(remove(&mut queue, failed));

        assert!(queue.is_empty());
    }

    #[test]
    fn taking_an_entry_returns_its_text_and_keeps_the_rest_in_order() {
        let mut queue = queue(&["first", "second", "third"]);
        let second = queue[1].id;

        assert_eq!(take(&mut queue, second).as_deref(), Some("second"));

        assert_eq!(texts(&queue), ["first", "third"]);
    }

    #[test]
    fn taking_picks_the_named_entry_among_identical_text() {
        let mut queue = queue(&["same", "same", "same"]);
        let middle = queue[1].id;
        let (first, last) = (queue[0].id, queue[2].id);

        assert_eq!(take(&mut queue, middle).as_deref(), Some("same"));

        assert_eq!(queue.iter().map(|prompt| prompt.id).collect::<Vec<_>>(),
                   [first, last]);
    }

    #[test]
    fn taking_an_absent_id_is_refused() {
        let mut queue = queue(&["first"]);

        assert_eq!(take(&mut queue, Uuid::new_v4()), None);

        assert_eq!(texts(&queue), ["first"]);
    }

    #[test]
    fn a_failed_entry_can_be_taken_for_editing() {
        let mut queue = queue(&["failed"]);
        queue[0].failed = true;
        let failed = queue[0].id;

        assert_eq!(take(&mut queue, failed).as_deref(), Some("failed"));

        assert!(queue.is_empty());
    }

    /// Edited text is a new prompt: it goes behind whatever is still
    /// waiting rather than reclaiming the place it was taken from.
    #[test]
    fn re_enqueuing_edited_text_puts_it_at_the_back() {
        let mut queue = queue(&["first", "second", "third"]);
        let first = queue[0].id;

        let text = take(&mut queue, first).expect("first should be editable");
        queue.push(QueuedPanelPrompt::new(format!("{text} (edited)"), PromptOrigin::User));

        assert_eq!(texts(&queue), ["second", "third", "first (edited)"]);
    }

    #[test]
    fn an_empty_composer_is_replaced_without_asking() {
        assert!(!needs_replace_confirmation(""));
    }

    #[test]
    fn a_whitespace_only_composer_is_not_work_worth_keeping() {
        assert!(!needs_replace_confirmation("  \n\t "));
    }

    #[test]
    fn typed_composer_text_has_to_be_confirmed_before_replacing() {
        assert!(needs_replace_confirmation("half a thought"));
        assert!(needs_replace_confirmation("  padded  "));
    }

    /// Declining the confirmation runs no queue operation at all, so the
    /// entry that was about to be edited is still queued and still first.
    #[test]
    fn declining_leaves_the_entry_queued() {
        let mut queue = queue(&["first", "second"]);
        let first = queue[0].id;

        if !needs_replace_confirmation("typed") {
            let _ = take(&mut queue, first);
        }

        assert_eq!(texts(&queue), ["first", "second"]);
        assert_eq!(queue[0].id, first);
    }

    #[test]
    fn retry_clears_the_failed_mark_and_keeps_the_place() {
        let mut queue = queue(&["first", "second"]);
        queue[1].failed = true;
        let second = queue[1].id;

        assert!(retry(&mut queue, second));

        assert_eq!(texts(&queue), ["first", "second"]);
        assert!(!queue[1].failed);
        assert_eq!(queue[1].id, second);
    }

    #[test]
    fn retry_ignores_an_entry_that_has_not_failed() {
        let mut queue = queue(&["first"]);
        let first = queue[0].id;

        assert!(!retry(&mut queue, first));
    }

    /// The row disappears when the prompt is sent, not when its turn ends.
    #[test]
    fn taking_the_head_removes_it_from_the_queue() {
        let mut queue = queue(&["first", "second"]);
        let first = queue[0].id;

        let taken = take_next(&mut queue).expect("the head is waiting");

        assert_eq!(taken.id, first);
        assert_eq!(texts(&queue), ["second"]);
    }

    #[test]
    fn a_failed_head_is_not_taken() {
        let mut queue = queue(&["failed", "second"]);
        queue[0].failed = true;

        assert_eq!(take_next(&mut queue), None);

        assert_eq!(texts(&queue), ["failed", "second"]);
    }

    #[test]
    fn taking_from_an_empty_queue_takes_nothing() {
        assert_eq!(take_next(&mut Vec::new()), None);
    }

    /// A failed delivery comes back as the same prompt, in front of what
    /// was queued behind it while it was out.
    #[test]
    fn a_failed_delivery_returns_to_the_head_marked() {
        let mut queue = queue(&["first", "second"]);
        let taken = take_next(&mut queue).expect("the head is waiting");
        let first = taken.id;
        queue.push(QueuedPanelPrompt::new("third".to_string(), PromptOrigin::User));

        return_failed(&mut queue, taken);

        assert_eq!(texts(&queue), ["first", "second", "third"]);
        assert_eq!(queue[0].id, first);
        assert!(queue[0].failed);
        assert_eq!(take_next(&mut queue),
                   None,
                   "the failed head blocks the queue");
    }
}
