# Proposal

## Why

A workspace with several agents running side-by-side is unreadable at a
glance: the only status signal is the state dot, whose color tells you what a
*running* agent is *doing* (Idle green / Working orange / Awaiting input red /
Error red) but has no way to say "not running at all". The user's most common
question — "which agents are actively working or thinking right now?" — can
only be answered by opening each row and reading a status lineamen.

## What Changes

- Add a **working/thinking indicator** — a glanceable, glanceable-at-a-glance
  visual mark on each agent's row in the workspace sidebar and on the
  dashboard's agent card that reflects the agent's live `Working` / `Idle` /
  `Awaiting input` / `Error` status as emitted by `activity-detection`, so the
  user can tell at a glance whether an agent is actively working, thinking,
  or waiting on input without reading a status string.
- The indicator SHALL be visually distinct from the existing status state dot:
  it is a separate surface, so a not-running agent stays clearly
  distinguishable from a running-but-idle one, and the dot's existing
  semantics (what a *running* agent is doing) are unchanged.
- The indicator SHALL appear wherever the toolbar/dashboard surfaces already
  show agent state, and SHALL follow the agent's live status transitions in
  real time, sharing one implementation so the sidebar row and the dashboard
  card never drift.

## Capabilities

### New Capabilities
- `activity-detection` status -> **working-indicator** (new): a single
  glanceable, glanceable-at-a-glance `working-indicator` capability surface
  that renders an agent's live `Working`/`Idle`/`Awaiting input`/`Error`
  status as a distinct visual mark in both the workspace sidebar row and the
  dashboard agent card, reusing `activity-detection`'s emitted status without
  changing what `activity-detection` requires.

### Modified Capabilities
<!-- No existing capability's requirements change: this is an additive,
     glanceable surface that consumes `activity-detection`'s status output. No
     delta spec is needed for the existing specs. -->

## Impact

- New UI component in the requestbar/dashboard layer shared by the workspace
  sidebar row (`agent-row`) and the dashboard agent card.
- Consumes `activity-detection`'s live status; no changes to
  `activity-detection`, `desktop-notifications`, or MCP-messaging
  requirements.
- No API or CRUD changes; purely additive to the agent row / dashboard card
  UI.
