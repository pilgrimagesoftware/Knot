# Spec Delta

## ADDED Requirements

### Requirement: User message chunks keep their metadata

A `user_message_chunk` SHALL be delivered with its `_meta` envelope, verbatim,
alongside its text, or with none when the agent sent none. Not every replayed
user message was typed by the user. An adapter may say who produced one: Claude
Code's origin is `_claude/origin`. A caller can only tell the two apart if the
metadata reaches it.

#### Scenario: An origin tag reaches the caller

- **WHEN** an agent sends a `user_message_chunk` whose `_meta` carries
  `_claude/origin` with kind `task-notification`
- **THEN** the delivered chunk carries that `_meta`

#### Scenario: A chunk without metadata

- **WHEN** an agent sends a `user_message_chunk` with no `_meta`
- **THEN** the delivered chunk carries its text and no metadata
