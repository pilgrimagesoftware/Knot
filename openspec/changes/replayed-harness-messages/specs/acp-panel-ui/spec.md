# Spec Delta

## ADDED Requirements

### Requirement: Harness-injected messages are not shown as the user's

A replayed user message chunk that the agent's harness injected, rather than
the user typing it, SHALL NOT be shown as a user message.

A chunk is injected when its `_meta["_claude/origin"].kind` is present and is
anything other than `human` or `channel`. When no origin is present, a chunk
is injected only if it consists entirely of one or more
`<task-notification>…</task-notification>` or
`<system-reminder>…</system-reminder>` blocks, with nothing but whitespace
between and around them. An origin, when present, decides in both directions:
a `human` or `channel` origin keeps a chunk the user's, whatever its text.

An injected chunk SHALL be shown as follows:

- A chunk holding a task notification SHALL become a compact notice row. Its
  text is "Background task finished: <summary>", using the notification's
  `<summary>` with its whitespace collapsed, or "Background task finished"
  when it has none.
- A chunk of `<system-reminder>` blocks only SHALL NOT be shown. The reminders
  are written for the model, and the live stream never shows them either.
- An origin-tagged chunk that is neither SHALL become a notice row reading
  "Automated message". The event is noted, but text written for the model is
  not shown as though someone had said it.

A notice row SHALL be a single line, smaller than a message and muted, and
SHALL be visually distinct from user, assistant, tool-call and error entries.
Its text SHALL come from localization.

An injected chunk SHALL NOT join the user message before it. This matters
because Claude Code appends reminders to a prompt as their own block, and the
prompt has to stay as the user typed it.

The same rule for when chunks are ignored as for the user's own prompts
applies: while a turn is in flight, a user message chunk is ignored.

This diverges from what `claude-agent-acp` does. Live, it drops these
messages; on replay, it sends them as the user's with no origin tag.

#### Scenario: A background task finished between turns

- **WHEN** a conversation that contains a `<task-notification>` user message
  with the summary "Tests passed" is replayed
- **THEN** no user message is shown for it, and a notice row reading
  "Background task finished: Tests passed" is shown where it was

#### Scenario: A prompt that mentions the tag

- **WHEN** a replayed user message reads `what is a <task-notification>?`
- **THEN** it is shown as the user's message

#### Scenario: A reminder appended to a prompt

- **WHEN** a replayed prompt arrives as a chunk of the user's text followed by
  a chunk that is only a `<system-reminder>` block
- **THEN** the prompt is shown as the user typed it, and the reminder is not
  shown

#### Scenario: The adapter tags the origin

- **WHEN** a replayed user chunk carries `_claude/origin` kind
  `task-notification`
- **THEN** it is not shown as the user's message, whatever its text

#### Scenario: A human origin wins over the text

- **WHEN** a replayed user chunk carries `_claude/origin` kind `human` and its
  text is a `<task-notification>` block
- **THEN** it is shown as the user's message
