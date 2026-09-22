# Proposal

## Why

`agent-lifecycle` currently lets a deactivated agent start again only by
being selected in the UI: "A deactivated agent SHALL NOT start again on its
own for as long as its workspace stays open; selecting it starts it." A
direct message is a deliberate, addressed request to one specific agent -
functionally the same kind of intent as clicking it - but today the message
just queues silently against a stopped agent, and nobody sees the reply (or
that there was no reply) until a person happens to select it. This makes
agent-to-agent delegation to a deactivated teammate unreliable.

## What Changes

- A direct `send` (not `broadcast`) whose recipient resolves to a
  deactivated agent SHALL activate that agent (start its session) as part of
  delivering the message, the same way selecting it in the UI does.
- Activation SHALL happen only when the message is actually going to be
  delivered - after the existing workspace/shell/companion routing checks
  pass - not on a rejected send.
- `broadcast` is out of scope: it addresses every eligible recipient in the
  workspace, not one specific agent, so it SHALL NOT activate anyone. This
  matches the "specifically targets another agent" framing of the request.
- The activation trigger from a message SHALL NOT itself count as the
  idle-time delivery nudge, and SHALL NOT bypass the nudge's existing
  turn-in-flight/permission-pending guard - an agent that was deactivated has
  neither, so the guard is moot for it, but the nudge and the activation are
  two separate effects of the same send and neither implies the other.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `mcp-messaging`: a direct send to a deactivated recipient activates it as
  part of delivery.
- `agent-lifecycle`: the "deactivated agent SHALL NOT start again on its own"
  requirement gains a second exception alongside UI selection - a direct
  message addressed to it.

## Impact

- `knot-mcp-tools/src/messaging.rs::send_message` - needs a mutable path to
  the agent store (currently `&AgentStore`) or an equivalent activation
  signal, since routing today only reads the store.
- `knot-messaging::routing::send` - the point that already knows delivery
  succeeded and who the recipient is.
- `knot-agents::AgentStore` - has `deactivate` (`store/lifecycle.rs`) but no
  store-level "activate"; starting a session today is UI-driven
  (`WorkspaceWindow::ensure_session` in `crates/knot/src/workspace_window/sessions.rs`).
  The MCP tool layer runs independently of any open window, so this change
  needs a way to request activation that a window can act on - see
  `design.md`.
