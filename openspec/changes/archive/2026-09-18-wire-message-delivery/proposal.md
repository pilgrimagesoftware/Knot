## Why

Agents can send each other messages. No agent can receive one.

`mcp-messaging` has specified the idle-time delivery nudge since the port
began, and every piece of it exists in the code: `knot-messaging` stores the
message and queues a delivery event, `should_inject_inbox_prompt` decides
whether to nudge, `CHECK_INBOX_PROMPT` is the text, `delivery_notice`
summarises it for the user, and `unread_counts_snapshot` counts unread per agent.

None of it is called. `MessageStore` never reaches the UI at all - it is
handed to the MCP catalog and nowhere else - so a message sent to an agent
lands in a store nothing reads.

The requirement is not missing. Its wiring is.

## What Changes

- **Deliver the nudge.** The workspace window's existing repaint poll gains
  the delivery check: an agent with an unread message it has not been nudged
  about, that is idle and has a live session with no turn in flight, is sent
  the "check your inbox" prompt.
- **Update the requirement for ACP.** It says the nudge is "injected into the
  recipient's terminal", which was true when non-shell agents ran in
  terminals. Under ACP-only launch they have no terminal; the nudge is a
  prompt in the agent's session. The guard is restated in the same terms:
  a turn in flight or an outstanding permission request, rather than a
  terminal's input-protection window.
- **Nudge once per message.** New, and necessary: the old wording fires on
  arrival, but a poll-driven check would re-fire on every idle moment until
  the agent reads its inbox.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `mcp-messaging`: restates the idle-time delivery nudge for agents that
  have a session rather than a terminal, and adds the once-per-message rule.

## Impact

- `crates/knot`: `MessageStore` is threaded to the workspace window and the
  repaint poll gains the delivery check.

Not addressed here: the sidebar has no unread badge. `layout_model` and
`AgentRow::unread_count` exist, are unit-tested, and have no caller - the
sidebar builds its rows directly from the store - so showing unread counts
is a UI change of its own rather than the map-population it looks like from
the outside.
- No change to `knot-messaging`, which already does its half correctly, nor
  to the MCP tools, `knot-git`, or `knot-discovery`.
- Nothing here changes what a message *is* or who may send one; the routing
  rules, workspace scoping and companion restrictions are untouched.
