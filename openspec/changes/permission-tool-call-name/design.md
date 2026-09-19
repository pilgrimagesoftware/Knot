# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- `PermissionRequest` today is `{ rpc_id, tool_call_id, options }`: the
  client extracts `toolCall.toolCallId` (with a flat `toolCallId` fallback);
  the panel renders "Permission requested for tool call {tool_call_id}".
- The panel already holds the call's human-readable label: `ToolCallCard`
  carries `title` ("Reading configuration file") and `kind`, populated from
  `ToolCallStart`, keyed by the same id the permission request names.
- `PanelState` keeps `pending_permission` and the message list in one place,
  so resolving a request to its card is a pure lookup, and
  `render_permission_prompt` renders both.
- Adaptors may order events however they like: nothing guarantees a
  `tool_call` update precedes the matching `request_permission`.

## Goals / Non-Goals

**Goals:**

- The user reads what they are approving: a title, not a munged id.
- A single resolution order, undefined-tail-safe, so the prompt can never
  come out blank.
- The wire change is strictly additive: a value that was absent stays absent,
  and adapters that send nothing new behave identically.

**Non-Goals:**

- No change to what is approved or denied, to the options, or to the
  decision round-trip.
- No change to the `permission-prompt-ui` capability's safety outcomes (risk
  tinting, blocking) - naming is a presentation concern on top of it.
- No rewording of the sentence beyond substituting the name for the id.

## Decisions

### `knot-acp` captures an optional title off the wire
`PermissionRequest` gains `tool_call_title: Option<String>`, parsed from
`toolCall.title` (mirroring how `toolCall.toolCallId` is found, with no flat
fallback needed - there is no flat precedent to keep). `None` when the agent
does not send one, so the wire stays a single source and the failure case is
a value that was never there, not a default that lies. The `acp-client` delta
spec pins both the addition and the None behavior.

### The panel resolves the display name in one ordered lookup
A pure `display_name` resolution over the request, in order:

1. the request's own `tool_call_title` (the adapter told us directly),
2. the matching `ToolCallCard`'s `title` when the id is known (the common
   case - the request names a call the panel is already showing),
3. the card's `kind` (executable, read, ...) when title is empty,
4. the raw id (today's behavior, now explicitly the last resort).

The first `Some` wins; the fallback chain is tested as a pure function, so
the renderer can't misorder it. Card lookup reuses the message list already
scanned for other per-card work.

### The renderer formats the name, not the sentence around it
`render_permission_prompt` keeps "Permission requested for …" and slots the
resolved name into it. Keeps the copy contract (`l10n::t`) simple and the
sentence single-sourced; the name is data, the sentence is glue.

### No card invention when nothing is known
If no title and no card exist, the id is shown - exactly today's behavior,
now as the explicit documented floor of the chain, satisfying the "never
blank" scenario without inventing a name the panel does not have.

## Risks / Trade-offs

- [An adapter sends a title that disagrees with the card title] → Precedence
  is fixed (wire first), documented, and the prompt is still human-readable
  either way; a card refresh later reconciles the conversation.
- [Card lookup cost per prompt] → A linear scan over a bounded message list
  per permission prompt, which is a user-paced event, not per-frame work; no
  indexing needed.
- [Changing `PermissionRequest` ripples through knot-terminal/panel constructs
  in tests] → The field is additive with a `Default`-friendly `Option`;
  existing construction sites compile unchanged.