## Why

The `acp-only-agent-launch` work added behaviour the contract does not
describe: when a shell agent's PTY process exits, the workspace window
removes that agent. It shipped because a dead pane reads as a hung one -
the terminal stops responding and nothing says why - but `agent-lifecycle`
only describes removal as a user-initiated action, so the implementation
currently has no contract to be right or wrong against.

This change writes the behaviour into `agent-lifecycle`, and fixes the
one place the implementation is broader than intended: the hand-off for
that branch recorded it as *companion*-exit behaviour, while the code
removes any shell agent whose process exits, companion or not.

## What Changes

- Specify that a shell agent whose terminal process exits is removed
  through the normal removal path (companions first, MCP unregister,
  session teardown, pane re-selection).
- Scope it to shell agents only. A non-shell agent has no PTY of its own
  under ACP-only launch, so nothing changes for those.
- Keep it distinct from user-initiated removal: an exit-driven removal
  SHALL NOT prompt for confirmation, because the process is already gone
  and there is nothing left to cancel.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `agent-lifecycle`: adds an exit-driven removal trigger for shell agents.

## Impact

- `crates/knot` (`workspace_window`): the behaviour already exists; this
  change pins it and adds the test it never had.
- No changes to `knot-core`, `knot-agents`, `knot-git`, `knot-discovery`,
  or the MCP server.
