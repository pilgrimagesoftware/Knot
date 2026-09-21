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

### Persist setup as the adapter's own config-option pairs

Persist the selection as `SavedAgent.session_config`, a map of ACP config-option id to selected value, with `#[serde(default)]` so a record written before the field existed loads as empty and leaves the adapter's defaults in place. Save on selection change. Replay the map onto the session after the handshake and before the first turn, so an in-flight turn keeps its original values.

Revised during implementation. The original plan was three named fields (model, permission mode, effort). Knot has no such vocabulary: all three are ACP Session Config Options the adapter declares at runtime, and the panel already locates them by matching id aliases (`mode`/`permission_mode`/`permission-mode`, `effort`/`reasoning`/`thought_level`, ...). Three named fields would have to guess an id to replay against, and would drop any fourth axis an adapter exposes. Storing the adapter's own pairs replays verbatim and generalizes.

This is also why the field is not an enum, despite the closed-vocabulary rule: the vocabulary is the adapter's and is open-ended, not Knot's.

Unlike `session_id`/`acp_session_id`, the map is not gated by `restore-conversation-on-launch` - setup is a preference, not conversation content.

Alternative: store one global preference. Rejected because agents can use different providers and setup choices.

### Persist compact mode as one settings preference

Add a boolean preference to the existing settings store, defaulting to false. This keeps the current UI behavior for existing users and avoids per-agent state unless product requirements later call for it.

Alternative: persist per-agent display mode. Rejected because the request describes an option for the panel, not a session-specific choice.

### Derive the summary rather than accumulate it

Compute the run and its counts from `PanelState::messages` at render time (`panel_state::summary`). A run is a maximal span of contiguous `PanelMessage::ToolCall` entries; any other message is a hard boundary. Counts are read off the live cards, so they move as results arrive with nothing to keep in step.

Revised during implementation. The original plan was a transient accumulator fed by the event stream. A derivation is strictly stronger for the same goal - "preserve ordered event data so changing display mode never loses details" - because there is no second copy of the data that can disagree with the messages, and switching modes cannot lose anything that was never stored separately. The only stored state is the set of runs the user has explicitly opened, keyed by the run's first tool-call id, mirroring how per-card collapse already stores overrides rather than states.

Compact mode also leaves the virtualized row model alone: every message keeps its own list row, the run's first call draws the summary line, and the rest draw nothing. Collapsing rows instead would have renumbered every index behind a run, discarding the list's measured heights and the reader's scroll position on every mode switch.

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
