# Proposal

## Why

After a Claude agent's session is resumed, messages that Claude Code's harness
injected show up as if the user had typed them (issue #551). The usual one is
the `<task-notification>` added when a background subagent or shell command
finishes. It renders as a user prompt bubble, raw XML included.

Claude Code stores these as `type: "user"` messages, with
`origin: {"kind": "task-notification"}` and a plain-string content.
`claude-agent-acp` handles them differently live and on replay:
- **Live,** it drops single-text user messages, so they never reach Knot.
- **On `session/load`,** it replays every stored user message as a
  `user_message_chunk`. It doesn't check the origin, and it doesn't attach the
  origin as `_meta`.

Knot kept only a chunk's text and folded every chunk into a user message. So a
message the live stream hides is shown as a prompt after a resume.

## What Changes

- `knot-acp` keeps a `user_message_chunk`'s `_meta`, so an origin tag an adapter
  sends reaches the panel.
- The panel classifies each replayed user chunk:
  - by `_meta["_claude/origin"].kind` when it is present: anything other than
    `human` or `channel` is injected;
  - otherwise, by content: a chunk made up entirely of `<task-notification>` and
    `<system-reminder>` blocks is injected.
- What it does with an injected chunk:
  - a task notification becomes a compact, muted row, "Background task
    finished: <summary>";
  - reminder-only chunks are hidden, as they are live;
  - an origin-tagged chunk of any other shape becomes an "Automated message"
    row.

  A prompt that only mentions a tag still renders as a prompt.
- The row text goes through `l10n::t`.

An upstream report to `claude-agent-acp`, asking it to skip or tag non-human
origins on replay as the live stream does, is out of scope.

## Impact

- `knot-acp`: `SessionUpdate::UserMessageChunk` gains `meta`.
- `knot`: `panel_state::harness` (the classifier), the `fold` path for user
  chunks, the new `PanelMessage::Notice` and how it renders.
- Specs: `acp-client`, `acp-panel-ui`.
