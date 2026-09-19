# Proposal

## Why

The tool-call card's header spends its scarce horizontal space on a status
word - "Pending", "Running…", "Done", "Failed" - set in the proportional font
beside a monospace title that already ellipsizes when long. A command or path
title loses its characters to a word the user already knows from how the card
looks: an icon colored by state says "still going" or "failed" at a glance
and frees the row for the title. The word is still worth having - it just
belongs in the icon's tooltip rather than on the row.

## What Changes

- Replace the tool-call header's inline status text with a **status icon**,
  colored by the call's state (info outline while running, danger on failure,
  neutral once done) and carrying the status word as a **tooltip**, so the
  state is still hearable/readable at hover.
- A status the panel does not recognize keeps today's behavior of showing the
  raw status text inline instead of inventing an icon for a state it cannot
  interpret ("a future value is more useful shown than hidden").
- The header's font rule survives: the title stays monospace, and the
  status's word - now the tooltip - stays proportional chrome text.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the tool-call header's status changes from an inline
  proportional word to a colored icon with the word in its tooltip, which
  updates the font-separation requirement and adds a status-indicator
  requirement.

## Impact

- `crates/knot` panel view: `render_tool_call_card` and `status_label` (which
  already maps statuses to words, and is reused verbatim for tooltips). The
  color mapping reuses `card_outline` / `PanelStyle`
  (`info_color`/`danger_color`/`border_color`), so no new colour decisions
  live in the view.
- No state changes and no crate below the UI window changes.