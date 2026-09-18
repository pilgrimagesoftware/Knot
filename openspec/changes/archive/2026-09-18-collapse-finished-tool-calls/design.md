## Context

See proposal.md — Why. What the panel already has:

- `ToolCallCard` carries `id`, `kind`, `title`, `status` and `content`, with
  `is_finished()` (completed or failed) and `failed()` already distinguishing
  the two ends.
- `render_tool_call_card` already draws a header row - icon, label, status -
  above the content. That header is exactly what a collapsed card is.
- `PanelState` already holds view state alongside the folded stream
  (`tracking`, `turn_active`), so per-card open/closed state has a home that
  is testable without GPUI.

## Goals / Non-Goals

**Goals:**

- A conversation whose length reflects what the user still needs to read.
- An automatic default that the user can always override, per card.

**Non-Goals:**

- Collapsing anything else. Assistant messages and user prompts stay as they
  are; this is about tool calls burying the reply.
- A global "collapse all" / "expand all". Worth considering once this is in
  use; adding it now guesses at a need.
- Remembering open/closed across sessions. A conversation reloaded from
  history starts clean, like the rest of the panel's view state.
- Truncating or summarising output. Hidden, never dropped.

## Decisions

### Failed calls stay expanded

The request says collapse them "when they're done", and a failed call is
done. It is also the card the user is looking for. Collapsing it would mean
the one result that needs reading is the one behind a control, while a
hundred successful reads sit neatly folded - the exact inversion of what the
feature is for.

This is the one place this change reads past the letter of the request, and
it is deliberate and reversible: `failed()` already exists, and flipping the
rule is one condition.

*Alternative considered:* collapse everything finished, uniformly. Simpler to
describe, worse to use - and the uniformity is only visible in the code, not
to the person reading a failed command's output.

### Collapsed-ness is computed, and only the user's overrides are stored

`PanelState` stores a map of tool-call id to the user's explicit choice.
Nothing stores "collapsed" for a card the user never touched - that is
derived:

    collapsed(card) = user_choice(card.id).unwrap_or(card.status == completed)

Storing only overrides is what makes "the user's choice outlives the
automatic one" fall out rather than needing to be maintained: a card the user
opened has an entry, and the entry does not care that the status later
changed. A stored boolean per card would have to be rewritten on every status
transition, and the rewrite is exactly where a card would slam shut on
completion.

The rule is a pure function over the card and the map, so every combination -
untouched and running, untouched and completed, opened then completed, closed
then completed, failed - is a unit test with no window in sight.

### The control is a disclosure chevron, and the whole header toggles

A chevron on the header row states the current state and which way it goes.
The header row itself is also the hit target, because a 12px chevron is a
poor one and the header carries no other action.

### Collapsing must not move the viewport

A card collapsing on completion changes the height of content that may be
above what the user is reading, which is how a list like this yanks the
reader somewhere else. The panel already has the mechanism to hold position -
its auto-scroll tracking - so the rule is the same one it already follows:
following the end means stay at the end, not following means stay where the
user is. Written into the spec because it is observable behaviour and the
kind of thing that is only noticed as "the panel jumps".

## Risks / Trade-offs

- **A user who wants to see everything now clicks a lot** → the override is
  per card and sticky, and a global expand-all is deliberately left until
  there is evidence it is wanted rather than guessed at.
- **A collapsed card hides that its output was enormous** → the header keeps
  the title and status; a size or line count in the header is a reasonable
  follow-up once the collapse itself is in use.
- **The delta cannot archive until `acp-agent-panel-ui` does** → recorded in
  the proposal and as the first task; `openspec validate` reports it too.

## Open Questions

None.
