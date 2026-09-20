//! Folds an ACP session's update stream into renderable panel state:
//! message list with streaming text accumulation, tool-call cards, and
//! pending permission state. Sibling to `terminal_view.rs` (which renders
//! a `Grid`) rather than a mode inside it - this folds a
//! `knot_acp::SessionEvent` stream into a different data model entirely.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md`.

use std::collections::HashMap;

use knot_acp::{
    ConfigOption, PermissionRequest, SessionEndCause, SessionEvent, SessionUpdate, ToolCallContent,
};
use serde_json::Value;

/// Pretty-prints a legacy `tool_call_result` payload for display, falling
/// back to its compact form if it somehow can't be re-serialized.
fn render_json(output: &Value) -> String {
    serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
}

/// One entry in the panel's message list: a user-sent prompt, streamed
/// assistant text, a tool call's card, or a failure the session reported
/// out of band - kept as distinct variants per `acp-panel-ui`'s "visually
/// distinguish user messages, assistant messages, and system/tool
/// content" requirement.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelMessage {
    User(String),
    Assistant(String),
    ToolCall(ToolCallCard),
    /// Something the session could not do: a refused prompt, a send that
    /// never reached the agent. These arrive as a JSON-RPC error response
    /// rather than a session update, so nothing in the event stream
    /// records them - without this entry the only trace of, say, an
    /// exhausted model quota was a line on stderr the user never sees.
    Error(String),
}

/// A tool call's rendered state: `kind` (execute, read, edit, ...), the
/// agent's human-readable `title`, its lifecycle `status`, and whatever
/// content has arrived so far - per the "Turn ends mid tool call"
/// scenario, the last known state is kept, never dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallCard {
    pub id: String,
    pub kind: String,
    pub title: String,
    /// `pending`, `in_progress`, `completed` or `failed` - defaulted to
    /// `pending` at start rather than left unknown, per the ACP spec.
    pub status: String,
    /// Output blocks as they arrive. A later update's `content` replaces
    /// this wholesale (the spec's updates carry the full current content,
    /// not a delta); an update with no `content` at all leaves it alone.
    pub content: Vec<ToolCallContent>,
}

impl ToolCallCard {
    /// Whether the call has finished, either way - the renderer shows an
    /// in-progress placeholder only while this is false.
    pub fn is_finished(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "failed")
    }

    pub fn failed(&self) -> bool {
        self.status == "failed"
    }

    /// Whether the call finished without failing - the one status that
    /// collapses by default, per `acp-panel-ui`'s "A finished tool call
    /// collapses to its header" requirement.
    pub fn succeeded(&self) -> bool {
        self.status == "completed"
    }
}

/// Folded state for one ACP session, per `acp-panel-ui`'s streaming
/// message and permission-prompt requirements.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PanelState {
    pub messages: Vec<PanelMessage>,
    /// Set by a `session/request_permission` event; sending further
    /// prompts SHALL be blocked while this is set (enforced by the caller
    /// that owns the `AcpSession`, not this pure state).
    pub pending_permission: Option<PermissionRequest>,
    /// Set once the session ends (normally or on error); `None` while live.
    pub ended: Option<SessionEndCause>,
    /// True from the user's prompt until the matching `TurnEnd`, per the
    /// response action bar design's "renders once streaming has ended"
    /// decision - the last message's action bar (copy needs stable text)
    /// and its track toggle are mutually exclusive on this flag.
    pub turn_active: bool,
    /// Whether the in-flight response should auto-scroll to follow new
    /// content, per the track toggle's per-response scope (design decision
    /// "Track toggle scope"). Reset to `true` at the start of each turn.
    pub tracking: bool,
    /// The agent's declared Session Config Options (permission mode,
    /// model, reasoning effort, ...), per ACP's stabilized mechanism -
    /// seeded from `session/new`/`session/load` and replaced wholesale on
    /// a `config_option_update` push or a `session/set_config_option`
    /// response.
    pub config_options: Vec<ConfigOption>,
    /// The user's explicit open/closed choice per tool-call id, and only
    /// those choices - a card the user has never touched has no entry and
    /// follows `is_collapsed`'s default. Storing overrides rather than a
    /// collapsed flag per card is what makes the choice outlive the
    /// automatic behaviour (design decision "Collapsed-ness is computed,
    /// and only the user's overrides are stored"): the entry does not care
    /// that the call's status later changed, so a card opened while running
    /// does not slam shut on completion.
    tool_call_collapsed: HashMap<String, bool>,
}

impl PanelState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies one event from the session's ordered stream.
    pub fn apply(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Update(update) => self.apply_update(update),
            SessionEvent::PermissionRequest(request) => self.pending_permission = Some(request),
            SessionEvent::Ended(cause) => self.ended = Some(cause),
        }
    }

    /// Clears the pending permission request once the caller has sent a
    /// decision back through the ACP client, per the "User denies a
    /// permission request" scenario: the prompt is replaced with its
    /// resolved state, not left pending.
    pub fn resolve_permission(&mut self) {
        self.pending_permission = None;
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
        let Some(collapsed) = self.tool_call(id).map(|card| self.is_collapsed(card)) else {
            return;
        };
        self.tool_call_collapsed.insert(id.to_string(), !collapsed);
    }

    fn apply_update(&mut self, update: SessionUpdate) {
        match update {
            SessionUpdate::TextDelta { text } => self.append_text(text),
            SessionUpdate::ToolCallStart {
                tool_call_id,
                kind,
                title,
                status,
                content,
            } => {
                self.messages.push(PanelMessage::ToolCall(ToolCallCard {
                    id: tool_call_id,
                    kind,
                    title,
                    status,
                    content,
                }));
            }
            SessionUpdate::ToolCallUpdate {
                tool_call_id,
                status,
                title,
                content,
            } => {
                if let Some(card) = self.tool_call_mut(&tool_call_id) {
                    // Absent fields mean "unchanged", per the ACP spec's
                    // partial updates - only overwrite what arrived.
                    if let Some(status) = status {
                        card.status = status;
                    }
                    if let Some(title) = title {
                        card.title = title;
                    }
                    if !content.is_empty() {
                        card.content = content;
                    }
                }
            }
            // Neither of these is a `sessionUpdate` kind any real agent
            // emits (results and diffs ride on `tool_call_update`'s
            // `content`), but `acp-client`'s streaming requirement names
            // them as distinguished events, so they fold into the same
            // card content rather than being dropped.
            SessionUpdate::ToolCallResult {
                tool_call_id,
                output,
            } => {
                if let Some(card) = self.tool_call_mut(&tool_call_id) {
                    card.content
                        .push(ToolCallContent::Text(render_json(&output)));
                }
            }
            SessionUpdate::Diff { path, diff } => {
                if let Some(PanelMessage::ToolCall(card)) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| matches!(m, PanelMessage::ToolCall(_)))
                {
                    card.content.push(ToolCallContent::Diff {
                        path,
                        old_text: None,
                        new_text: diff,
                    });
                }
            }
            // A turn-end carries only a stop reason; the tool call/message
            // list already reflects the turn's last known state and needs
            // no change beyond ending the turn (which flips the last
            // response from "track toggle" to "response action bar").
            SessionUpdate::TurnEnd { .. } => self.turn_active = false,
            SessionUpdate::ConfigOptionUpdate { config_options } => {
                self.config_options = config_options;
            }
            SessionUpdate::Unknown { .. } => {}
        }
    }

    /// Appends `text` to the current streaming message, starting a new one
    /// only when the last message isn't plain text (e.g. it's a tool call,
    /// or this is the first message) - per the "Rapid successive text
    /// deltas" scenario: deltas accumulate into one message, not several.
    fn append_text(&mut self, text: String) {
        if let Some(PanelMessage::Assistant(existing)) = self.messages.last_mut() {
            existing.push_str(&text);
        } else {
            self.messages.push(PanelMessage::Assistant(text));
        }
    }

    fn tool_call(&self, id: &str) -> Option<&ToolCallCard> {
        self.messages
            .iter()
            .rev()
            .find_map(|message| match message {
                PanelMessage::ToolCall(card) if card.id == id => Some(card),
                _ => None,
            })
    }

    fn tool_call_mut(&mut self, id: &str) -> Option<&mut ToolCallCard> {
        self.messages
            .iter_mut()
            .rev()
            .find_map(|message| match message {
                PanelMessage::ToolCall(card) if card.id == id => Some(card),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn text(text: &str) -> SessionEvent {
        SessionEvent::Update(SessionUpdate::TextDelta {
            text: text.to_string(),
        })
    }

    #[test]
    fn successive_text_deltas_accumulate_into_one_message() {
        let mut state = PanelState::new();

        state.apply(text("Hel"));
        state.apply(text("lo, "));
        state.apply(text("world"));

        assert_eq!(
            state.messages,
            vec![PanelMessage::Assistant("Hello, world".to_string())]
        );
    }

    #[test]
    fn user_message_is_recorded_and_does_not_merge_with_assistant_text() {
        let mut state = PanelState::new();

        state.push_user_message("hello".to_string());
        state.apply(text("hi there"));

        assert_eq!(
            state.messages,
            vec![
                PanelMessage::User("hello".to_string()),
                PanelMessage::Assistant("hi there".to_string())
            ]
        );
    }

    /// An agent that answers a prompt with an error (an exhausted quota,
    /// a dead transport) sends no `TurnEnd`, so the failure has to both
    /// show in the conversation and release the composer.
    #[test]
    fn a_failed_prompt_is_recorded_as_an_error_and_ends_the_turn() {
        let mut state = PanelState::new();

        state.push_user_message("hello".to_string());
        state.push_error("The agent could not answer: quota exhausted".to_string());

        assert_eq!(
            state.messages,
            vec![
                PanelMessage::User("hello".to_string()),
                PanelMessage::Error("The agent could not answer: quota exhausted".to_string())
            ]
        );
        assert!(
            !state.turn_active,
            "a failed turn must not keep the composer blocked"
        );
    }

    fn tool_call_start(id: &str, kind: &str) -> SessionEvent {
        SessionEvent::Update(SessionUpdate::ToolCallStart {
            tool_call_id: id.to_string(),
            kind: kind.to_string(),
            title: String::new(),
            status: "pending".to_string(),
            content: Vec::new(),
        })
    }

    fn tool_call_update(
        id: &str, status: Option<&str>, content: Vec<ToolCallContent>,
    ) -> SessionEvent {
        SessionEvent::Update(SessionUpdate::ToolCallUpdate {
            tool_call_id: id.to_string(),
            status: status.map(str::to_string),
            title: None,
            content,
        })
    }

    #[test]
    fn tool_call_lifecycle_builds_one_card() {
        let mut state = PanelState::new();

        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));
        state.apply(tool_call_update(
            "tc1",
            Some("completed"),
            vec![ToolCallContent::Text("done".to_string())],
        ));

        assert_eq!(
            state.messages,
            vec![PanelMessage::ToolCall(ToolCallCard {
                id: "tc1".to_string(),
                kind: "execute".to_string(),
                title: String::new(),
                status: "completed".to_string(),
                content: vec![ToolCallContent::Text("done".to_string())],
            })]
        );
    }

    /// A finished call with no content must not keep reading as running -
    /// the card renderer keys its in-progress placeholder on this, and
    /// keying it on "no output yet" instead left completed cards stuck
    /// showing "Running…".
    #[test]
    fn a_completed_tool_call_is_finished_even_with_no_content() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

        let PanelMessage::ToolCall(card) = &state.messages[0] else {
            panic!("expected a tool call card");
        };
        assert!(card.is_finished());
        assert!(!card.failed());
        assert!(card.content.is_empty());
    }

    /// Per the ACP spec, every field but `toolCallId` is optional in an
    /// update: one carrying only content must not blank out the status.
    #[test]
    fn a_content_only_update_leaves_the_status_alone() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
        state.apply(tool_call_update(
            "tc1",
            None,
            vec![ToolCallContent::Text("late output".to_string())],
        ));

        let PanelMessage::ToolCall(card) = &state.messages[0] else {
            panic!("expected a tool call card");
        };
        assert_eq!(card.status, "completed");
        assert_eq!(
            card.content,
            vec![ToolCallContent::Text("late output".to_string())]
        );
    }

    #[test]
    fn edit_tool_call_attaches_a_diff() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "edit"));

        state.apply(tool_call_update(
            "tc1",
            Some("completed"),
            vec![ToolCallContent::Diff {
                path: "src/lib.rs".to_string(),
                old_text: Some("old".to_string()),
                new_text: "new".to_string(),
            }],
        ));

        let PanelMessage::ToolCall(card) = &state.messages[0] else {
            panic!("expected a tool call card");
        };
        assert_eq!(
            card.content,
            vec![ToolCallContent::Diff {
                path: "src/lib.rs".to_string(),
                old_text: Some("old".to_string()),
                new_text: "new".to_string(),
            }]
        );
    }

    /// The legacy `diff` session update carries no `toolCallId`, so it
    /// attaches to the most recent card - kept working because
    /// `acp-client`'s streaming requirement names diffs as a distinguished
    /// event, even though no shipping agent emits one this way.
    #[test]
    fn a_standalone_diff_update_attaches_to_the_last_card() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "edit"));

        state.apply(SessionEvent::Update(SessionUpdate::Diff {
            path: "src/lib.rs".to_string(),
            diff: "-old\n+new".to_string(),
        }));

        let PanelMessage::ToolCall(card) = &state.messages[0] else {
            panic!("expected a tool call card");
        };
        assert_eq!(
            card.content,
            vec![ToolCallContent::Diff {
                path: "src/lib.rs".to_string(),
                old_text: None,
                new_text: "-old\n+new".to_string(),
            }]
        );
    }

    #[test]
    fn turn_end_mid_tool_call_keeps_the_cards_last_known_state() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

        state.apply(SessionEvent::Update(SessionUpdate::TurnEnd {
            stop_reason: "end_turn".to_string(),
        }));

        let PanelMessage::ToolCall(card) = &state.messages[0] else {
            panic!("expected a tool call card");
        };
        assert_eq!(card.status, "in_progress");
        assert!(!card.is_finished());
        assert!(card.content.is_empty());
    }

    #[test]
    fn permission_request_is_pending_until_resolved() {
        let mut state = PanelState::new();
        let request = PermissionRequest {
            rpc_id: json!(1),
            tool_call_id: "tc1".to_string(),
            options: Vec::new(),
        };

        state.apply(SessionEvent::PermissionRequest(request.clone()));
        assert_eq!(state.pending_permission, Some(request));

        state.resolve_permission();
        assert_eq!(state.pending_permission, None);
    }

    #[test]
    fn session_end_is_recorded() {
        let mut state = PanelState::new();

        state.apply(SessionEvent::Ended(SessionEndCause::ProcessExited {
            code: Some(1),
        }));

        assert_eq!(
            state.ended,
            Some(SessionEndCause::ProcessExited { code: Some(1) })
        );
    }

    #[test]
    fn user_message_starts_a_tracked_turn_and_turn_end_ends_it() {
        let mut state = PanelState::new();

        state.push_user_message("hello".to_string());
        assert!(state.turn_active);
        assert!(state.tracking);

        state.apply(SessionEvent::Update(SessionUpdate::TurnEnd {
            stop_reason: "end_turn".to_string(),
        }));
        assert!(!state.turn_active);
    }

    #[test]
    fn scrolling_away_clears_tracking_and_a_new_turn_resets_it() {
        let mut state = PanelState::new();
        state.push_user_message("hello".to_string());

        state.clear_tracking();
        assert!(!state.tracking);

        state.push_user_message("again".to_string());
        assert!(state.tracking);
    }

    #[test]
    fn toggle_tracking_flips_the_flag() {
        let mut state = PanelState::new();
        state.push_user_message("hello".to_string());

        state.toggle_tracking();
        assert!(!state.tracking);
        state.toggle_tracking();
        assert!(state.tracking);
    }

    /// The card at `index`, for the collapse tests below.
    fn card(state: &PanelState, index: usize) -> &ToolCallCard {
        let PanelMessage::ToolCall(card) = &state.messages[index] else {
            panic!("expected a tool call card");
        };
        card
    }

    #[test]
    fn an_untouched_running_call_is_expanded() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        assert!(
            !state.is_collapsed(card(&state, 0)),
            "a pending call must stay open"
        );

        state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));
        assert!(
            !state.is_collapsed(card(&state, 0)),
            "a running call's output is what the user is waiting on"
        );
    }

    #[test]
    fn an_untouched_completed_call_is_collapsed() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

        assert!(state.is_collapsed(card(&state, 0)));
    }

    /// The one place this reads past "collapse when they're done": a
    /// failure is done, and it is also the card the user opened the
    /// conversation to read.
    #[test]
    fn an_untouched_failed_call_stays_expanded() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("failed"), Vec::new()));

        assert!(!state.is_collapsed(card(&state, 0)));
    }

    #[test]
    fn a_call_opened_while_running_stays_open_when_it_completes() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

        // Running calls render open, so a toggle closes one; toggling
        // again is the user explicitly opening it.
        state.toggle_tool_call("tc1");
        state.toggle_tool_call("tc1");
        state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

        assert!(
            !state.is_collapsed(card(&state, 0)),
            "a call the user opened must not slam shut on completion"
        );
    }

    #[test]
    fn a_call_closed_while_running_stays_closed_when_it_completes() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

        state.toggle_tool_call("tc1");
        assert!(
            state.is_collapsed(card(&state, 0)),
            "a running call can be closed and goes on streaming out of sight"
        );

        state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
        assert!(state.is_collapsed(card(&state, 0)));
    }

    #[test]
    fn a_call_closed_while_running_stays_closed_when_it_fails() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("in_progress"), Vec::new()));

        state.toggle_tool_call("tc1");
        state.apply(tool_call_update("tc1", Some("failed"), Vec::new()));

        assert!(
            state.is_collapsed(card(&state, 0)),
            "the user's choice outranks the automatic expansion of a failure"
        );
    }

    #[test]
    fn opening_one_call_leaves_the_others_alone() {
        let mut state = PanelState::new();
        for id in ["tc1", "tc2"] {
            state.apply(tool_call_start(id, "read"));
            state.apply(tool_call_update(id, Some("completed"), Vec::new()));
        }

        state.toggle_tool_call("tc1");

        assert!(!state.is_collapsed(card(&state, 0)));
        assert!(state.is_collapsed(card(&state, 1)));
    }

    #[test]
    fn toggling_an_unknown_call_records_nothing() {
        let mut state = PanelState::new();

        state.toggle_tool_call("no-such-call");

        assert!(state.tool_call_collapsed.is_empty());
    }

    /// Nothing about the open/closed choices is persisted: the overrides
    /// live in the panel's view state, so a reloaded conversation starts
    /// from the automatic behaviour again.
    #[test]
    fn a_reloaded_conversation_collapses_a_call_the_previous_one_had_open() {
        let mut state = PanelState::new();
        state.apply(tool_call_start("tc1", "execute"));
        state.apply(tool_call_update("tc1", Some("completed"), Vec::new()));
        state.toggle_tool_call("tc1");
        assert!(!state.is_collapsed(card(&state, 0)));

        let mut reloaded = PanelState::new();
        reloaded.apply(tool_call_start("tc1", "execute"));
        reloaded.apply(tool_call_update("tc1", Some("completed"), Vec::new()));

        assert!(reloaded.is_collapsed(card(&reloaded, 0)));
    }

    #[test]
    fn config_option_update_replaces_the_declared_options() {
        let mut state = PanelState::new();
        let option = ConfigOption {
            id: "mode".to_string(),
            name: "Mode".to_string(),
            category: Some("mode".to_string()),
            kind: "select".to_string(),
            current_value: json!("code"),
            options: Vec::new(),
        };

        state.apply(SessionEvent::Update(SessionUpdate::ConfigOptionUpdate {
            config_options: vec![option.clone()],
        }));

        assert_eq!(state.config_options, vec![option]);
    }
}
