//! The folded state itself: what one ACP session's panel holds, and the
//! questions and edits the UI puts to it - the prompt it just sent, whether
//! auto-scroll is on, which tool calls are open.
//!
//! How the session's event stream changes it is `super::fold`.

use std::collections::HashMap;
use std::collections::HashSet;

use knot_acp::ConfigOption;
use knot_acp::PermissionRequest;
use knot_acp::SessionEndCause;
use knot_processes::ShellRunState;
use uuid::Uuid;

use super::message::PanelMessage;
use super::message::ShellCard;
use super::message::ToolCallCard;

/// Folded state for one ACP session, per `acp-panel-ui`'s streaming
/// message and permission-prompt requirements.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelState {
    /// The agent-reported context-window token usage, if the ACP adapter
    /// supports `usage_update` notifications.
    pub context_usage:              Option<(u64, u64)>,
    pub messages:                   Vec<PanelMessage>,
    /// Set by a `session/request_permission` event; sending further
    /// prompts SHALL be blocked while this is set (enforced by the caller
    /// that owns the `AcpSession`, not this pure state).
    pub pending_permission:         Option<PermissionRequest>,
    /// Set once the session ends (normally or on error); `None` while live.
    pub ended:                      Option<SessionEndCause>,
    /// True from the user's prompt until the matching `TurnEnd`, per the
    /// response action bar design's "renders once streaming has ended"
    /// decision - the last message's action bar (copy needs stable text)
    /// and its track toggle are mutually exclusive on this flag.
    pub turn_active:                bool,
    /// Whether the in-flight response should auto-scroll to follow new
    /// content, per the track toggle's per-response scope (design decision
    /// "Track toggle scope"). Reset to `true` at the start of each turn.
    pub tracking:                   bool,
    /// The agent's declared Session Config Options (permission mode,
    /// model, reasoning effort, ...), per ACP's stabilized mechanism -
    /// seeded from `session/new`/`session/load` and replaced wholesale on
    /// a `config_option_update` push or a `session/set_config_option`
    /// response.
    pub config_options:             Vec<ConfigOption>,
    /// The user's explicit open/closed choice per tool-call id, and only
    /// those choices - a card the user has never touched has no entry and
    /// follows `is_collapsed`'s default. Storing overrides rather than a
    /// collapsed flag per card is what makes the choice outlive the
    /// automatic behaviour (design decision "Collapsed-ness is computed,
    /// and only the user's overrides are stored"): the entry does not care
    /// that the call's status later changed, so a card opened while running
    /// does not slam shut on completion.
    pub(super) tool_call_collapsed: HashMap<String, bool>,
    /// The runs the user has opened in compact mode, keyed by the id of
    /// the run's first tool call. Only the opened ones are stored, for the
    /// same reason `tool_call_collapsed` stores only overrides: a run the
    /// user has never touched follows the mode's default, and a run that
    /// grows another call afterwards does not forget it was opened.
    tool_run_expanded:              HashSet<String>,
    /// Pull request URLs seen in this session's tool-call text, canonical
    /// and de-duplicated, waiting to be drained by the window that owns the
    /// agent store.
    ///
    /// Buffered here rather than recorded here because this state is pure -
    /// it knows nothing of which agent it belongs to, and nothing of the
    /// store. The window drains it beside `take_dirty`, which is the poll
    /// that already runs whenever this state has changed.
    pull_request_urls:              Vec<String>,
}

impl PanelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the pull request URLs seen since the last call.
    ///
    /// Drained rather than read so the window records each sighting once,
    /// however often it polls. Recording is idempotent per agent anyway, so a
    /// URL handed over twice costs nothing - this just keeps the buffer from
    /// growing for the life of the session.
    pub fn take_pull_request_urls(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pull_request_urls)
    }

    /// Note any pull request URLs in `text`, ignoring ones already buffered.
    ///
    /// A tool-call update replaces a card's content wholesale, so the same
    /// text is folded in more than once; without the check the buffer would
    /// grow with every re-render of a card that happens to mention one.
    pub(super) fn note_pull_request_urls(&mut self, text: &str) {
        for url in knot_core::pull_request_url::scan_pull_request_urls(text) {
            let url = url.to_string();
            if !self.pull_request_urls.contains(&url) {
                self.pull_request_urls.push(url);
            }
        }
    }

    /// Clears the pending permission request once the caller has sent a
    /// decision back through the ACP client, per the "User denies a
    /// permission request" scenario: the prompt is replaced with its
    /// resolved state, not left pending.
    pub fn resolve_permission(&mut self) {
        self.pending_permission = None;
    }

    /// Resolves the most useful human-readable name for a permission request.
    pub fn display_name(&self, request: &PermissionRequest) -> String {
        request.tool_call_title
               .as_deref()
               .filter(|title| !title.is_empty())
               .or_else(|| {
                   self.tool_call(&request.tool_call_id)
                       .map(|card| card.title.as_str())
                       .filter(|title| !title.is_empty())
               })
               .or_else(|| {
                   self.tool_call(&request.tool_call_id)
                       .map(|card| card.kind.as_str())
                       .filter(|kind| !kind.is_empty())
               })
               .unwrap_or(&request.tool_call_id)
               .to_owned()
    }

    /// Records a prompt the user just sent, so it shows in the
    /// conversation - the ACP stream itself never echoes it back. Starts a
    /// new turn: the next response tracks by default until the user
    /// scrolls away or the turn ends.
    pub fn push_user_message(&mut self, text: String) {
        self.messages.push(PanelMessage::User(text));
        self.turn_active = true;
        self.tracking = true;
    }

    /// Records a failure the agent reported instead of a turn - a prompt
    /// the adapter answered with a JSON-RPC error, say. Ends the turn:
    /// the error response *is* the turn's outcome, no `TurnEnd` follows
    /// it, and leaving `turn_active` set would lock the composer out of
    /// sending anything else for the rest of the session.
    pub fn push_error(&mut self, text: String) {
        self.messages.push(PanelMessage::Error(text));
        self.turn_active = false;
    }

    /// Records a `!` command the user just submitted, per
    /// `panel-shell-passthrough`.
    ///
    /// Deliberately does *not* touch `turn_active` or `tracking` the way
    /// `push_user_message` does: a shell command is not a turn. Running one
    /// while the agent is answering must leave that answer exactly as it
    /// was, and running one while the agent is idle must not make the panel
    /// think a turn has begun.
    pub fn push_shell_command(&mut self, card: ShellCard) {
        self.messages.push(PanelMessage::Shell(card));
    }

    /// Updates the card for `id` with the run's latest state, reporting
    /// whether the card was found.
    ///
    /// A missing card is not an error: the conversation can be cleared out
    /// from under a run that is still going, and the run has no way to know.
    pub fn update_shell_command(&mut self, id: Uuid, run: ShellRunState) -> bool {
        let Some(card) = self.shell_card_mut(id)
        else {
            return false;
        };

        card.absorb(run);
        true
    }

    /// The card for `id`, if the conversation still holds it.
    ///
    /// Only the tests ask this: production reaches a card through
    /// `update_shell_command` or the pending list, which is what keeps the
    /// promotion rule in one place.
    #[cfg(test)]
    pub fn shell_card(&self, id: Uuid) -> Option<&ShellCard> {
        self.messages.iter().find_map(|message| match message {
                                PanelMessage::Shell(card) if card.id == id => Some(card),
                                _ => None,
                            })
    }

    pub(super) fn shell_card_mut(&mut self, id: Uuid) -> Option<&mut ShellCard> {
        self.messages.iter_mut().find_map(|message| match message {
                                    PanelMessage::Shell(card) if card.id == id => Some(card),
                                    _ => None,
                                })
    }

    /// Drops one card's pending result so no later prompt carries it.
    pub fn discard_shell_result(&mut self, id: Uuid) {
        if let Some(card) = self.shell_card_mut(id) {
            card.discard();
        }
    }

    /// The cards, in submission order, whose results are waiting for a
    /// prompt.
    pub fn pending_shell_results(&self) -> Vec<&ShellCard> {
        self.messages
            .iter()
            .filter_map(|message| match message {
                PanelMessage::Shell(card) if card.is_pending() => Some(card),
                _ => None,
            })
            .collect()
    }

    /// Marks every pending result as delivered, which is what sending a
    /// prompt that carried them means.
    pub fn mark_shell_results_shared(&mut self) {
        for message in &mut self.messages {
            if let PanelMessage::Shell(card) = message
               && card.is_pending()
            {
                card.mark_shared();
            }
        }
    }

    /// Turns off auto-scroll for the in-flight response, per the track
    /// toggle's "detect user-initiated scroll away from bottom" scenario.
    pub fn clear_tracking(&mut self) {
        self.tracking = false;
    }

    /// Toggles auto-scroll for the in-flight response.
    pub fn toggle_tracking(&mut self) {
        self.tracking = !self.tracking;
    }

    /// Sets auto-scroll directly - jumping to the end of the conversation
    /// resumes following it, rather than flipping whatever it was.
    pub fn set_tracking(&mut self, tracking: bool) {
        self.tracking = tracking;
    }

    /// Whether `card` renders collapsed: the user's choice if they have
    /// made one, otherwise the default - collapsed once the call has
    /// succeeded, expanded while it runs and expanded if it failed (the
    /// failure output is what the user is reading the conversation for).
    pub fn is_collapsed(&self, card: &ToolCallCard) -> bool {
        self.tool_call_collapsed
            .get(&card.id)
            .copied()
            .unwrap_or_else(|| card.succeeded())
    }

    /// Records the user opening or closing one tool call, flipping
    /// whichever state it currently renders in. Unknown ids are ignored:
    /// the override is only meaningful against a card that exists.
    pub fn toggle_tool_call(&mut self, id: &str) {
        let Some(collapsed) = self.tool_call(id).map(|card| self.is_collapsed(card))
        else {
            return;
        };
        self.tool_call_collapsed.insert(id.to_string(), !collapsed);
    }

    /// Whether the run headed by `head_id` is drawn as its individual
    /// cards despite compact mode - the inspection path the spec requires
    /// compact rendering to keep.
    pub fn is_tool_run_expanded(&self, head_id: &str) -> bool {
        self.tool_run_expanded.contains(head_id)
    }

    /// Opens or closes one run in compact mode.
    pub fn toggle_tool_run(&mut self, head_id: String) {
        if !self.tool_run_expanded.remove(&head_id) {
            self.tool_run_expanded.insert(head_id);
        }
    }

    pub(super) fn tool_call(&self, id: &str) -> Option<&ToolCallCard> {
        self.messages
            .iter()
            .rev()
            .find_map(|message| match message {
                PanelMessage::ToolCall(card) if card.id == id => Some(card.as_ref()),
                _ => None,
            })
    }

    pub(super) fn tool_call_mut(&mut self, id: &str) -> Option<&mut ToolCallCard> {
        self.messages
            .iter_mut()
            .rev()
            .find_map(|message| match message {
                PanelMessage::ToolCall(card) if card.id == id => Some(card.as_mut()),
                _ => None,
            })
    }
}
