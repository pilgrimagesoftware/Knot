---
paths:
  - "crates/knot-acp/**"
  - "crates/knot-mcp/**"
  - "crates/knot-mcp-tools/**"
---

# Model a protocol from its spec, not from inference

Four bugs on `acp-only-agent-launch` (PR #123) had the same shape: the code
modelled MCP or ACP from what seemed plausible rather than from the spec, and
failed silently. If a feature "does nothing", suspect this first.

- `tools/list` serialized `input_schema`; MCP requires `inputSchema`. A
  validating client drops every tool in the list, so Knot's whole tool set
  was invisible. It looked like a port collision because a different MCP
  server in the user's own config answered to the same tool names.
- ACP has no `tool_call_result` or `diff` session update. A call's output
  arrives as `content` on `tool_call`/`tool_call_update`, so every card read
  "Running…" forever.
- ACP has no `turn_end` update. A turn ends by responding to
  `session/prompt`. The response was discarded, so `turn_active` stayed true
  from the first prompt onward and Send was dead.
- `session/request_permission` nests `params.toolCall.toolCallId`; the code
  read a flat `params.toolCallId`. The existing test asserted the wrong
  shape, so it passed against the bug.

When adding or changing a message shape, cite the spec section in the code
or test, and build test fixtures from the spec's own examples, not from the
struct being tested.
