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

/// One prompt waiting to be delivered to an agent.
///
/// `in_flight` marks the entry the pump has handed to the agent - it stays
/// in the queue until its result comes back, so the row keeps its place
/// while the turn runs. `failed` marks one whose delivery returned an
/// error; it holds its position until the user retries or deletes it,
/// rather than being retried automatically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QueuedPanelPrompt {
    pub(crate) id:        Uuid,
    pub(crate) text:      String,
    pub(crate) failed:    bool,
    pub(crate) in_flight: bool,
}

impl QueuedPanelPrompt {
    pub(crate) fn new(text: String) -> Self {
        Self { id: Uuid::new_v4(),
               text,
               failed: false,
               in_flight: false }
    }

    /// Whether the user may delete this entry.
    ///
    /// An in-flight entry has already been submitted and the agent is
    /// answering it, so removing its row would leave a turn running with
    /// nothing on screen to explain it - the spec's "deletion does not
    /// affect active work" cuts both ways.
    pub(crate) fn is_deletable(&self) -> bool {
        !self.in_flight
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
    if !queue[index].is_deletable() {
        return false;
    }
    queue.remove(index);
    true
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

/// Applies a delivery result: a delivered prompt leaves the queue, a failed
/// one stays and is marked so the user can retry or delete it.
pub(crate) fn complete(queue: &mut Vec<QueuedPanelPrompt>, id: Uuid, delivered: bool) {
    let Some(index) = position(queue, id)
    else {
        return;
    };
    if delivered {
        queue.remove(index);
    }
    else {
        queue[index].in_flight = false;
        queue[index].failed = true;
    }
}

fn position(queue: &[QueuedPanelPrompt], id: Uuid) -> Option<usize> {
    queue.iter().position(|prompt| prompt.id == id)
}

/// The localized name of a queued entry's state, for the row's status icon.
///
/// The icon carries the state visually and the row's only text is the
/// prompt itself, so this is the sole thing that tells a screen reader
/// whether the entry is waiting or stuck.
pub(crate) fn queued_status_label(failed: bool) -> String {
    knot_core::l10n::t(if failed {
                           "panel.failed"
                       }
                       else {
                           "panel.queued"
                       })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue(texts: &[&str]) -> Vec<QueuedPanelPrompt> {
        texts.iter()
             .map(|text| QueuedPanelPrompt::new((*text).to_string()))
             .collect()
    }

    fn texts(queue: &[QueuedPanelPrompt]) -> Vec<&str> {
        queue.iter().map(|prompt| prompt.text.as_str()).collect()
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
    fn an_in_flight_entry_is_not_deletable() {
        let mut queue = queue(&["running", "waiting"]);
        queue[0].in_flight = true;
        let running = queue[0].id;

        assert!(!remove(&mut queue, running));

        assert_eq!(texts(&queue), ["running", "waiting"]);
    }

    #[test]
    fn deleting_while_a_turn_runs_leaves_the_running_entry_alone() {
        let mut queue = queue(&["running", "waiting"]);
        queue[0].in_flight = true;
        let waiting = queue[1].id;

        assert!(remove(&mut queue, waiting));

        assert_eq!(texts(&queue), ["running"]);
        assert!(queue[0].in_flight);
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

    #[test]
    fn a_delivered_prompt_leaves_the_queue() {
        let mut queue = queue(&["first", "second"]);
        let first = queue[0].id;
        queue[0].in_flight = true;

        complete(&mut queue, first, true);

        assert_eq!(texts(&queue), ["second"]);
    }

    #[test]
    fn a_failed_delivery_stays_and_is_marked() {
        let mut queue = queue(&["first", "second"]);
        let first = queue[0].id;
        queue[0].in_flight = true;

        complete(&mut queue, first, false);

        assert_eq!(texts(&queue), ["first", "second"]);
        assert!(queue[0].failed);
        assert!(!queue[0].in_flight);
    }

    #[test]
    fn a_result_completes_its_own_entry_among_identical_text() {
        let mut queue = queue(&["same", "same"]);
        let second = queue[1].id;
        queue[1].in_flight = true;

        complete(&mut queue, second, true);

        assert_eq!(queue.len(), 1);
        assert_ne!(queue[0].id, second);
    }

    #[test]
    fn a_result_for_a_deleted_entry_is_ignored() {
        let mut queue = queue(&["first"]);
        let gone = Uuid::new_v4();

        complete(&mut queue, gone, false);

        assert_eq!(texts(&queue), ["first"]);
        assert!(!queue[0].failed);
    }
}
