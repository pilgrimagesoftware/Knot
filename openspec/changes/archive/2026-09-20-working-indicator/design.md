# Design

## Context

See proposal.md — Why. Behavior contract lives in
`specs/working-indicator/spec.md`. Current implementation facts that shape
the approach:

- `activity-detection` already emits a live per-agent status — exactly one of
  `Working` / `Idle` / `Awaiting input` / `Error` — with recorded change
  timestamps, over a hook/acp-driven path plus terminal-poll fallback. The
  indicator is a pure consumer of that output; it adds NO new detection or
  status semantics.
- Two surfaces already render agent state and must stay consistent: the
  workspace **sidebar agent row** (`agent-list-ui`) and the **dashboard agent
  card** (`dashboard`). Both currently key off the `agent-lifecycle` row
  model and a single shared status dot whose "value" only ever describes a
  *running* agent (no "not running" value exists).
- The underlying renderer is gpui-kit (Rust, SwiftUI-free), so all animation
  is driven in-app, not by CSS or system spinners. We do not get an animated
  GIF/spinner for free.

## Goals / Non-Goals

**Goals:**
- A single shared `working-indicator` UI component whose behavior is
  identical in the sidebar row and the dashboard card, seeded from one
  source of truth so the two can never drift.
- A glanceable, glanceable-at-a-glance visual that distinguishes
  `Working` vs `Idle` vs `Awaiting input` vs `Error` without reading text.
- A clear visual distinction between "not running at all" and "running but
  idle", since that is the question the existing state dot cannot answer.

**Non-Goals:**
- Changing `activity-detection`, `activity-detection` internals, or MCP
  layers — the indicator consumes existing status output only (per spec's
  skip on modified capabilities).
- Replacing the existing state dot or its semantics; the dot keeps reporting
  what a running agent is *doing*. The indicator is additive.
- Touch/gesture-driven interaction, audio, or force-touch variants — out of
  scope for a glanceable mark.
- Localization of the indicator itself — it is iconography + animation, not
  a text string (color-blind-safe distinction is still required).

## Decisions

### D1: Single shared component consumed by both surfaces
**Decision:** One `WorkingIndicator` gpui model/view in the shared UI layer,
instantiated by the sidebar row and the dashboard card; both subscribe to the
same per-agent status source (`activity-detection`'s live status).

**Rationale:** Two independent implementations are the exact late-signal
they exist to prevent: a status change lands in the row weeks after the
card, and nobody notices until someone compares them. One component + one
subscription means both surfaces update on the same tick. This directly
serves the spec's "SHALL NOT drift" requirement.

**Alternatives considered:**
- *Two bespoke indicators per surface* — rejected: reintroduces drift, the
  precise risk the capability exists to eliminate.
- *Reuse the state dot for everything* — rejected: the dot has no
  "not running" value)Skip, so a deactivated/passive agent would read as
  "idle running agent", exactly the conflation the spec calls out.

### D2: Indicator = status-driven animation, not a static glyph
**Decision:** The indicator is an in-app animated mark driven by status:
- `Working` → animated (pulsing/spinning) "on the move" state
- `Idle` → steady, subdued, but *present* state (visibly different from
  Working; agent is "there but not doing")
- `Awaiting input` → distinct attention-drawing pulse (visibly different
  from Working and Idle)
- `Error` → distinct error styling
- **not running** → entirely absent/off ("bare"), so it reads as "this
  agent isn't live" instead of "live but idle"

**Rationale:** Animation is the only dimension that conveys
"doing something right now" without text and without the user reading.
Not-running = off is the only way to keep the "not running" case visually
separate from "running and idle", which the synchronous dot cannot do.
Chosen over a fixed icon so a status change reads in peripheral vision.

**Alternatives considered:**
- *Static color-only icon* — rejected: fails color-blind / reduced-palette
  requirement; a static mark cannot show "actively changing".
- *Text label ("Working"/"Thinking")* — rejected: the whole point is
  not needing to read; see Why in proposal.md.
- *Reuse an OS spinner (NSProgressIndicator-style)* — rejected: gpui-kit
  has no system spinner; the animation must be drawn in-app anyway.

### D3: Source of truth = `activity-detection`'s live status stream
**Decision:** The indicator subscribes to the same per-agent status stream
that already drives the row/card (the `Working`/`Idle`/`Awaiting input`/
`Error` state + timestamps), and reflects changes immediately. It does not
start a second timer or re-detect anything.

**Rationale:** `activity-detection` owns "what is this agent doing right
now" — duplicating that in the indicator would fork the truth and let them
disagree. Consuming the emitted status keeps the indicator reusing live
behavior per the spec, with zero new detecti
**Alternatives considered:**
- *Indicator runs its own idle/working detection* — rejected: two engines
  for one status is the drift source; also duplicates the hook/acp plumbing.

### D4: Placement as a leading mark on the row/card, aligned with the dot
**Decision:** The indicator sits in the same visual zone the state dot
already occupies on each surface (leading edge of the avatar row), sized
to the surface, and is layered so the existing dot retains its own
semantics alongside.

**Rationale:** Puts the new signal where the user already looks for
"what is this agent doing", minimising re-learning, and keeps both signals
on the same row for comparison.Skip (alternatives) — only one sensible spot.

### D5: Animation engine go through gpui-kit timer/animation primitives
**Decision:** Animation uses gpui-kit's timer/animation primitives (a
per-frame tick or the kit's animation helper), not a bespoke
`tokio`/async loop.

**Rationale:** gpui-kit is the shared render/animation layer mapped from
SwiftUI+AppKit; a hand-rolled tokio loop for blinking would bypass it and
duplicate timing. Consistent with the stack mapping in AGENTS.md.

**Alternatives considered:** hand-rolled tokio animation loop — rejected,
bypasses gpui-kit and drifts from the rendering layer.

## Risks / Trade-offs

- **[Animation is attention-grabbing by design]** → Risk: over-animation
  at scale (many agents) becomes noise/strobing, fatiguing the user.
  *Mitigation*: subdued motion by default, Working vs Idle vs Awaiting kept
  distinct, and animation limited to status-change transitions, not
  continuous motion — matching the "animate on change" spec wording.
- **[Peripheral-vision reliance]** → Risk: a user who cannot perceive
  motion (or who has motion disabled) loses the signal.
  *Mitigation*: the indicator is never motion-only — each state also has a
  distinct static mark/shape and the existing dotted states remain, so
  information is not solely in the animation.
- **[Shared component = shared failure]** → Risk: a bug in the one shared
  component takes down both the sidebar row and the dashboard card.
  *Mitigation*: it is a pure view; single source of truth; unit-test the
  component in isolation (scenarios in spec are testable).
- **[Not-running vs running-idle is an overloaded signa
  Risk: users may still conflate "off" with "broken".
  *Mitigation*: "off" is visually an absence (nothing), distinct from every
  coloured state, and the existing error/awaiting states remain — the
  indicator never claims "error" when the agent is merely not running.

## Migration Plan

- Additive UI: no data-model, API, or dependency changes; no migration
  needed for stored settings (indicator is derived from live status, not
  persisted).
- The indicator renders only when status is present; a not-running agent
  simply renders the "off" state — safe to ship and roll back by removing
  the component from the shared layer with no state migration.

## Open Questions

- None blocking. Surface-composition details (exact placement/inset on the
  avatar cluster, spacing between dot and indicator across the two
  surfaces) can be tuned during implementation without changing the spec.
