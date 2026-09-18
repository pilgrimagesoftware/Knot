## Why

A tool call card tells you almost nothing at a glance. Its title and its
status render in the same font, so a shell command and the word "Done" look
like one run-on line. Its outline is the same grey whether the call is
running, finished or failed - except for failures, which get a red border
from a hardcoded value that ignores the theme.

The result is a conversation where the two states worth noticing, a call
still running and a call that failed, look like the eleven that succeeded.

## What Changes

- **The title renders monospace.** It is a command, a path or an identifier;
  the project renders those monospace everywhere else.
- **The status text renders proportional.** It is the panel's own words about
  the call, not something the agent produced, and separating the two fonts is
  what makes the header readable as two things instead of one string.
- **The outline says what state the call is in**: the theme's danger colour
  for failed, its info colour for pending or running, and the panel's
  ordinary neutral border for completed.
- **Success gets no colour of its own.** A conversation whose every finished
  call is outlined in green is one where nothing stands out, which defeats
  the point of outlining anything.
- **Colours come from the theme**, replacing the card's hardcoded values, so
  the card follows a theme change like everything around it.

## Capabilities

### Modified Capabilities
- `acp-panel-ui`: adds what a tool call card's fonts and outline say.

### New Capabilities
(none)

## Impact

- `crates/knot`: `panel_view`'s tool-call card, and the theme colours it
  needs reaching it the way its monospace family already does.
- Nothing below the UI. No change to what a tool call *is* or how its status
  arrives.
- **Sequencing:** `acp-panel-ui` is not yet under `openspec/specs/` - it
  belongs to `acp-agent-panel-ui`, implemented but unarchived - so this
  delta can only archive after that one. The same applies to
  `collapse-finished-tool-calls`, which touches the same card header; the
  two compose (that one adds a control to the header, this one restyles it)
  but both wait on the same archive.
