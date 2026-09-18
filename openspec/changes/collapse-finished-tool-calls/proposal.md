## Why

A long turn buries its own answer. Every tool call the agent makes renders as
a full card with its output - file contents, command output, diffs - and a
turn with a dozen reads leaves the agent's actual reply somewhere above or
below a wall of material the user has already stopped caring about. The
output mattered while the call was running; a second after it succeeds it is
scroll.

## What Changes

- A tool call that **succeeds** collapses to its header once finished: icon,
  title and status stay, its output is hidden.
- A tool call that is **still running** stays expanded - its output is what
  the user is waiting for.
- A tool call that **failed** stays expanded. This is the deliberate reading
  of "when they're done": a failure is done, but it is also the one card the
  user opened the conversation to read, and putting it behind a control makes
  the important card the one that takes an extra click.
- Every card gains a **control to open and close it**, whatever its status,
  so the automatic behaviour is a default rather than something to fight.
- **The user's choice wins.** A card the user has opened or closed keeps that
  state, including when the call finishes - so expanding a running call to
  watch it does not slam shut the moment it succeeds.

## Capabilities

### Modified Capabilities
- `acp-panel-ui`: adds the collapse behaviour, its control, and the rules for
  which state a card is in.

### New Capabilities
(none)

## Impact

- `crates/knot`: `PanelState` gains the per-call open/closed choices and the
  pure rule that reads them; `panel_view` renders the control and hides
  content when collapsed.
- No change to `knot-acp` or anything that reads the ACP stream. Content is
  hidden, never dropped, so nothing about what the panel *holds* changes.
- **Sequencing:** `acp-panel-ui` is not yet in `openspec/specs/` - it belongs
  to the change `acp-agent-panel-ui`, which is implemented but unarchived.
  This change's delta adds requirements to that capability and can only
  archive after it.
