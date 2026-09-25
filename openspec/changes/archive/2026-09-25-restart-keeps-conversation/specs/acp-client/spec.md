# Spec Delta

## MODIFIED Requirements

### Requirement: Streaming session updates
The system SHALL deliver `session/update` notifications to the caller as an
ordered stream distinguishing at least: text deltas, user message chunks,
tool-call start/update/result, diffs, and turn-end (with a stop reason). Text
deltas for the same message SHALL be deliverable in arrival order without the
caller needing to re-request the message.

User message chunks (`user_message_chunk`) are how an agent replays the
user's side of a conversation during `session/load`. They SHALL be delivered
as their own kind, not dropped as unknown, so a caller can show a restored
conversation's prompts between its replies.

#### Scenario: Turn ends mid tool call
- **WHEN** the agent reports a turn-end update while a tool call is still
  open
- **THEN** the system SHALL surface the tool call's last known state alongside
  the turn-end event rather than dropping it

#### Scenario: A replayed prompt is delivered as the user's
- **WHEN** the agent sends a `user_message_chunk` update while loading a
  session
- **THEN** the system SHALL deliver it as a user message chunk carrying its
  text
