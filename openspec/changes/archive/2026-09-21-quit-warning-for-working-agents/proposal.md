# Proposal

## Why

Quitting while agents are working can terminate active commands and lose useful progress without giving the user a chance to reconsider. The app should warn only when work is active, while keeping quit immediate when all agents are idle.

## What Changes

- Detect whether any managed agent is currently working when the app receives a quit request.
- Show a confirmation dialog before quitting if at least one agent is working.
- Identify the number of working agents in the warning when more than one is active.
- Continue quitting when the user confirms and cancel quitting when the user declines.
- Allow explicit quit paths to bypass the warning after confirmation without showing it again.

## Capabilities

### New Capabilities

- `quit-warning`: Protects active agent work from accidental application exit.

### Modified Capabilities

- None.

## Impact

- `crates/knot/` application and window quit handling.
- Agent state aggregation for identifying active work.
- Localized warning title, message, and actions.
- Quit-flow tests; no changes to agent execution or persistence.
