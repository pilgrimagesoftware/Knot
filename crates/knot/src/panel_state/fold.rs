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
                                           content, } => {
                self.messages
                    .push(PanelMessage::ToolCall(ToolCallCard { id: tool_call_id,
                                                                kind,
                                                                title,
                                                                status,
                                                                content }));
            }
            SessionUpdate::ToolCallUpdate { tool_call_id,
                                            status,
                                            title,
                                            content, } => {
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
            SessionUpdate::ToolCallResult { tool_call_id,
                                            output, } => {
                if let Some(card) = self.tool_call_mut(&tool_call_id) {
                    card.content
                        .push(ToolCallContent::Text(render_json(&output)));
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
