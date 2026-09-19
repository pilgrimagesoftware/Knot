# Proposal

## Why

A collapsed tool call card is supposed to be a one-line summary - and it is
anything but. The header's title (`flex_1().min_w_0()`) wraps onto a second
or third line when it is long, so a "collapsed" call can sit two or three
rows tall and push the conversation down by more than the card that earned
it. Collapsing buys next to nothing if the fold still grows with the title.

## What Changes

- A **collapsed** tool call card's header renders as a **single line**: the
  title truncates with an ellipsis instead of wrapping, and the status
  indicator stays pinned on the row.
- **Expanded** cards are unchanged - an open card already has real content
  below its header, so a wrapping title there is not what the user is
  trying to shrink.
- The collapse toggle and the rest of the header (chevron, kind icon, status
  indicator, tooltip) behave exactly as they do now.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the collapse behavior gains a fixed one-line, ellipsized
  header, so a collapsed card's height no longer depends on its title.

## Impact

- `crates/knot` panel view: `render_tool_call_card` applies the
  nowrap/ellipsis styling to the title element while `collapsed` is true,
  matching the pattern `detail_line` already uses for one-line rows. The
  status indicator is `flex_shrink_0` and stays exactly where it is.
- No state changes and no crate below the UI window changes.