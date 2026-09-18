## Why

`acp-panel-ui` (change `acp-agent-panel-ui`, implementation complete, not
yet archived) specifies the panel's message stream, tool-call cards, and
permission prompts, but not its chrome: per-tool-call icons and the fixed
control bars a chat panel needs day to day. Zed's agent panel is the
reference for the look and control set we need: per-tool-call icons, a
clean message list, and a fixed control bar (response actions at the
bottom of history; context/permission/model/effort/send controls in the
input area). Without a spec this UI has no contract to implement or test
against.

## What Changes

- Add tool-call icons to the message list: each tool invocation renders
  with an icon identifying the tool, replacing any current text-only
  rendering.
- Add a response action bar under each agent response: copy response,
  scroll to the originating user message, scroll to top of history.
- Add a "track" toggle on the response action bar: while enabled, the
  panel auto-scrolls to follow new output as the agent streams a response.
- Add an input-area control bar: add-context (files/images), permission
  mode selector, model selector, effort selector, send button.
- Add an expand control on the input area that grows it into a larger
  multi-line editor and collapses it back.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: adds tool-call icons to the message list, a response
  action bar (copy, scroll-to-user-message, scroll-to-top, track toggle),
  and an input-area control bar (add-context, permission mode, model,
  effort, send, expand/collapse).

## Impact

- `crates/knot` (GPUI Kit UI shell): extends `panel_view.rs`/`panel_state.rs`
  with new components and event wiring for send, model/effort/permission
  selection, context attachment, scroll tracking, and input expand/collapse.
- No changes to `knot-core`, `knot-git`, `knot-discovery`, or the MCP
  server; this is UI-only.
- Depends on `acp-agent-panel-ui` being archived (or archives together)
  so `acp-panel-ui` exists under `openspec/specs/` before this change's
  delta is applied.
