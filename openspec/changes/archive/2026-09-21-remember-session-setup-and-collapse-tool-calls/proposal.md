# Proposal

## Why

Users repeatedly configure the same model, permission mode, and reasoning effort for agent sessions, while the panel currently exposes every tool call as a separate visual item. Persisting session setup removes repetitive configuration, and an optional compact tool-call presentation keeps long agent turns readable without hiding activity.

## What Changes

- Persist each agent session's selected model, permissions, and effort settings and restore them when the session is reopened.
- Preserve backward compatibility for saved agents and settings that lack the new fields.
- Add an option to collapse all tool-call cells in a turn into one updating summary line.
- Keep the summary line updated with counts and activity such as tools called and files edited, and return to normal rendering at the next prompt or other non-tool output.
- Retain the existing per-tool-call rendering as the default and as an alternate mode.
- Keep failed or otherwise important tool activity represented in the summary and preserve access to detailed tool-call content where the UI supports expansion.

## Capabilities

### New Capabilities

- `session-setup-persistence`: Persist and restore per-agent session model, permission mode, and reasoning effort.
- `collapsed-tool-call-summary`: Optionally aggregate tool-call activity into a single live summary line.

### Modified Capabilities

- `settings-persistence`: Add durable session setup and the tool-call display preference with decode-tolerant defaults.
- `acp-panel-ui`: Add the compact tool-call summary rendering mode and its lifecycle behavior.

## Impact

- Settings and saved-session serialization in the Rust port.
- ACP panel state, tool-call event aggregation, and conversation rendering.
- Settings UI for choosing the tool-call display mode.
- No new external dependencies or network APIs are required.

## Non-goals

- Changing the available model, permission, or effort choices.
- Removing detailed tool-call history or changing tool execution semantics.
- Persisting transient summary counts as conversation content.
