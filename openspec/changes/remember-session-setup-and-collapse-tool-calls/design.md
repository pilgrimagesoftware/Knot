# Design

## Context

See proposal.md - Why. Existing settings persistence already owns durable agent and UI settings, while the ACP panel receives ordered text and tool-call events and renders tool calls independently.

## Goals / Non-Goals

**Goals:**

- Store session setup beside the agent's durable session configuration with tolerant decoding.
- Aggregate only contiguous tool-call events into a transient presentation model.
- Make compact mode a persisted preference while keeping the existing card path intact.
- Preserve ordered event data so changing display mode never loses details.

**Non-Goals:**

- Changing ACP protocol payloads or tool execution.
- Persisting transient summary rows as conversation history.
- Replacing the existing per-call collapse behavior when compact mode is disabled.

## Decisions

### Persist setup as optional session fields

Add optional model, permission mode, and effort fields to the persisted agent/session settings, with decode defaults matching current control defaults. Save on selection change. Resolve the effective setup when creating the next turn, so an in-flight turn keeps its original values.

Alternative: store one global preference. Rejected because agents can use different providers and setup choices.

### Persist compact mode as one settings preference

Add a boolean preference to the existing settings store, defaulting to false. This keeps the current UI behavior for existing users and avoids per-agent state unless product requirements later call for it.

Alternative: persist per-agent display mode. Rejected because the request describes an option for the panel, not a session-specific choice.

### Aggregate in panel presentation state

Maintain a transient summary accumulator keyed by the current response segment. Start a summary on the first tool event, update counts as tool results arrive, and close it when text, a prompt boundary, or another non-tool event arrives. Keep the underlying tool-call records available for inspection and mode switching.

Alternative: rewrite the event stream before storage. Rejected because it would make detailed rendering and replay lossy.

### Count from structured tool metadata

Derive edited/read files and command counts from the existing tool-call kind, input, and result metadata. If metadata is unavailable, show the call count and omit unsupported secondary counts rather than guessing.

Alternative: parse display strings. Rejected because titles and output formats are unstable.

## Risks / Trade-offs

- [Legacy data lacks new fields] -> Use serde defaults and migration tests; no existing settings or agent record may fail to load.
- [Event ordering splits a summary unexpectedly] -> Treat any non-tool event as a hard boundary and test interleaved text/tool sequences.
- [Compact mode hides failure details] -> Keep failed call records in panel state and expose an inspection affordance from the summary.
- [Counts require tool-specific metadata] -> Make secondary counts best-effort and always show total calls.

## Migration Plan

1. Add optional persisted fields and the compact-mode preference with defaults.
2. Ship panel aggregation behind the disabled-by-default preference.
3. Validate legacy settings and saved-agent fixtures before enabling the setting in the UI.

Rollback removes the UI toggle and aggregation path; unknown persisted fields remain safely ignored by older builds.
