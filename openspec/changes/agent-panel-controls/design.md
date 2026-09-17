## Context

`panel_view.rs` renders `PanelState` (from `panel_state.rs`) as a chat-like
panel: `PanelMessage::{User, Assistant, ToolCall(ToolCallCard)}`, plus a
pending-permission prompt and an ended-session banner. `ToolCallCard`
already carries the ACP `kind` used to pick a diff view for edit-kind
calls — see proposal.md for motivation, and specs/acp-panel-ui/spec.md
for the exact requirements this design implements.

There is no existing input-area component in `crates/knot`; agents are
currently prompted by typing into the embedded terminal or (for
ACP-managed agents) a plain send field feeding `session/prompt`. This
design adds the first structured input bar.

## Goals / Non-Goals

**Goals:**
- Icon-per-tool-kind mapping and fallback, reusing `ToolCallCard.kind`.
- A response action bar and input-area control bar as gpui-kit components,
  matching the `knot-ui-conventions` icon-button/ghost-variant pattern.
- Wire selectors (permission, model, effort) to existing session state
  (`knot-acp` session capabilities / `PermissionDecision`) rather than
  introducing new state machines.

**Non-Goals:**
- No new ACP protocol surface — model/effort/permission selection use
  whatever `knot-acp` already exposes for a session; if an agent's
  adapter doesn't support switching a given axis (e.g. effort) mid-session,
  the selector is disabled, not stubbed with fake options.
- No terminal-view changes; this is panel-only chrome, per acp-panel-ui's
  existing "terminal remains available" requirement.
- No persistence of per-conversation input-area state (expanded/collapsed,
  attached-context) across app restarts.

## Decisions

- **Icon mapping**: keyed by `ToolCallCard.kind` (already an enum from
  `knot-acp`), not by tool name string-matching — kinds are a closed,
  already-parsed set, so a name-based lookup would just duplicate that
  enum with more failure modes. Unknown/future kinds fall back to a
  generic icon via the enum's exhaustive match default arm.
- **Track toggle scope**: per-response, not global — each in-flight
  response gets its own toggle so tracking one doesn't force-scroll the
  panel away from a response the user is re-reading further up.
- **Control bar placement**: response action bar renders as part of
  `render_message` for `PanelMessage::Assistant` (only once the message is
  no longer streaming, so copy has stable content); the input-area control
  bar is a new sibling component under `render_panel`, not part of the
  message list.
- **Selectors are session-scoped, not per-message**: permission mode,
  model, and effort selectors mutate session-level state that applies
  starting with the next `session/prompt` call, matching how `knot-acp`
  already threads permission mode through the session rather than the
  request.

## Risks / Trade-offs

- Some ACP adapters may not report model/effort options at all →
  selectors render disabled with a tooltip explaining why, rather than
  hiding (keeps the control bar layout stable across agents).
- Auto-scroll (track toggle) fighting user-initiated scroll is a common
  chat-UI bug → detect "scrolled away from bottom" via the scroll
  container's offset, not a debounced heuristic, to avoid flicker.
