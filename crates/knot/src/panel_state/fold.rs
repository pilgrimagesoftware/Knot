//! Folding an ACP session's update stream into [`PanelState`]: one event at a
//! time, in the order the session reported them.

use knot_acp::SessionEvent;
use knot_acp::SessionUpdate;
use knot_acp::ToolCallContent;

use super::message::PanelMessage;
use super::message::ToolCallCard;
use super::message::render_json;
use super::state::PanelState;

impl PanelState {
    /// Applies one event from the session's ordered stream.
    pub fn apply(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Update(update) => self.apply_update(update),
            SessionEvent::PermissionRequest(request) => self.pending_permission = Some(request),
            SessionEvent::Ended(cause) => self.ended = Some(cause),
        }
    }

    /// Scan a tool call's content for pull request URLs as it is folded in.
    ///
    /// Here rather than over the rendered card because this runs once per
    /// event, on the ACP drain, and the card is redrawn every frame. Only
    /// text is scanned: a diff's body is a patch, not output an agent
    /// printed.
    fn note_pull_requests_in(&mut self, content: &[ToolCallContent]) {
        for item in content {
            if let ToolCallContent::Text(text) = item {
                self.note_pull_request_urls(text);
            }
        }
    }

    fn apply_update(&mut self, update: SessionUpdate) {
        match update {
            SessionUpdate::Usage { used, size } if size > 0 => {
                self.context_usage = Some((used, size));
            }
            SessionUpdate::Usage { .. } => {}
            SessionUpdate::TextDelta { text } => self.append_text(text),
            SessionUpdate::ToolCallStart { tool_call_id,
                                           kind,
                                           title,
                                           status,
                                           content,
                                           raw_input,
                                           meta, } => {
                self.note_pull_requests_in(&content);
                self.messages
                    .push(PanelMessage::ToolCall(Box::new(ToolCallCard { id: tool_call_id,
                                                                         kind,
                                                                         title,
                                                                         status,
                                                                         content,
                                                                         raw_input,
                                                                         meta })));
            }
            SessionUpdate::ToolCallUpdate { tool_call_id,
                                            status,
                                            title,
                                            content,
                                            raw_input,
                                            meta, } => {
                self.note_pull_requests_in(&content);
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
                    // Same rule, and the one that matters most here: the
                    // completion update carries a *narrower* `_meta` than the
                    // start did - Claude Code's stamps `subagent: true` on the
                    // start and only `toolName` on the finish. Overwriting
                    // unconditionally would erase the marker a recognizer
                    // reads, so an absent field leaves what is already known.
                    if raw_input.is_some() {
                        card.raw_input = raw_input;
                    }
                    if meta.is_some() {
                        card.meta = meta;
                    }
                }
            }
            // Neither of these is a `sessionUpdate` kind any real agent
            // emits (results and diffs ride on `tool_call_update`'s
            // `content`), but `acp-client`'s streaming requirement names
            // them as distinguished events, so they fold into the same
            // card content rather than being dropped.
            SessionUpdate::ToolCallResult { tool_call_id,
                                            output, } => {
                let rendered = render_json(&output);
                self.note_pull_request_urls(&rendered);
                if let Some(card) = self.tool_call_mut(&tool_call_id) {
                    card.content.push(ToolCallContent::Text(rendered));
                }
            }
            SessionUpdate::Diff { path, diff } => {
                if let Some(PanelMessage::ToolCall(card)) =
                    self.messages
                        .iter_mut()
                        .rev()
                        .find(|m| matches!(m, PanelMessage::ToolCall(_)))
                {
                    card.content.push(ToolCallContent::Diff { path,
                                                              old_text: None,
                                                              new_text: diff });
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
        }
        else {
            self.messages.push(PanelMessage::Assistant(text));
        }
    }
}
