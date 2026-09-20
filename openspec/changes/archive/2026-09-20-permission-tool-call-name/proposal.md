# Proposal

## Why

The permission prompt says "Permission requested for tool call
`tool_call_a1b2…`" - an opaque id the agent minted, not a name anyone can act
on. The user is being asked to allow or deny an action they cannot read, on
an agent whose own tool-call cards already label the same call with a
human-readable title ("Reading configuration file"). The prompt should name
what it is asking about; the raw id is a fallback, not a headline.

## What Changes

- The permission prompt names the tool call by a **human-readable title**
  when one is available: "Permission requested for *Reading configuration
  file*".
- The title is resolved in order: the one the wire carried (if the adapter
  sends it) → the title of the panel's known tool-call card for that id →
  the call's kind → the raw id, so the prompt degrades gracefully and never
  shows blank.
- `knot-acp` captures an optional `toolCall.title` on the wire as a first
  source, because a permission request can arrive before any tool-call
  update has rendered a card.
- The decision the user makes is unchanged; only the prompt's wording
  changes.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the permission-prompt requirement gains naming - the prompt
  identifies the tool call by its human-readable title rather than its opaque
  id.
- `acp-client`: the permission-request event carries the tool call's optional
  title when the agent reports one, alongside the existing id and options.

## Impact

- `crates/knot-acp`: `PermissionRequest` gains an optional `tool_call_title`,
  parsed additively from `params.toolCall.title` (the flat spelling stays a
  fallback, and adapters that send neither behave exactly as before).
- `crates/knot`: `panel_state`/`panel_view` resolve the display name and
  render it in `render_permission_prompt`.
- No change to what is approved or denied, and no change to any wire message
  the client sends.