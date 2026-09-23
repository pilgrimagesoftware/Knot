# Design

## Context

See proposal.md - Why.

`prepare_frame` already owns this decision for the composer. The machinery is
in two pieces and both carry over:

- `composer_focus::showing_composer` - a pure function over facts read out of
  the store, answering which composer the frame *about to be drawn* will show.
  It is pure so that its branch order can be tested against
  `render/content.rs`'s without a window.
- `focus_showing_composer` - compares that answer against
  `WorkspaceWindow::focused_composer`, takes focus only on a change, and
  stores the answer either way, so a frame skipped for an open dialog is not
  replayed as a transition later.

The terminal pane differs from the composer in one way that matters: its
element only exists once the session has a grid. Until then `render/content.rs`
draws a "Starting terminal…" placeholder, and `self.terminal_focus` is tracked
by no element on screen.

## Goals / Non-Goals

**Goals:**

- Selecting a shell agent leaves the user able to type, with no click.
- One latch covers both kinds of pane, so switching between a Panel-mode and a
  Terminal-mode agent is a transition in both directions.
- The decision stays a pure function that can be tested without a window.

**Non-Goals:**

- Per-agent terminal focus handles. One handle is enough: only the selected
  agent's pane is rendered, and the terminal keeps no caret state of its own
  the way a composer keeps its draft.
- Any change to what the terminal does with keys once it has focus, or to how
  clicking the pane focuses it.
- Restoring focus to the terminal after a dialog closes. The composer does not
  do that either, and for the same stated reason.

## Decisions

### Widen the decision to "which input target", not "which composer"

`showing_composer` becomes a function returning which of the pane's input
targets the next frame will show:

```
enum FocusTarget { Composer(Uuid), Terminal(Uuid) }
```

and `focused_composer` becomes a latch over `Option<FocusTarget>`.

The alternative - a second, parallel `showing_terminal` predicate with its own
latch field - looks smaller but is wrong at the one point that matters.
Selecting a Panel-mode agent and then a Terminal-mode one has to read as a
transition for the terminal, and two independent latches each see only their
own half. One latch over a sum type gets that for free, and keeps the branch
order in a single function that can still be compared against
`render/content.rs`.

### Gate the terminal target on the grid, not on the selection

`SelectedAgentFacts` gains `has_live_grid`, and `FocusTarget::Terminal` is
only returned when it is true.

Focusing on the selection frame instead would focus a handle that is not in
the element tree. `prepare_frame`'s own fallback - "if nothing holds focus,
focus the window root" - then moves focus to the root on the same frame, and
because the latch has already stored the answer, no later frame retries. The
user would be back to clicking, intermittently, depending on how fast the
session spawned. Gating on the grid makes the frame it appears on the
transition.

Reading it is a map lookup and a `try_lock`-free `lock()` on the session,
which `render/content.rs` does on the same frame anyway.

### Leave `sessions.rs`'s latch reset as it is

Removing an agent already clears the latch when it names that agent. That
stays correct for either variant once the field holds a `FocusTarget`, so the
comparison there widens to "names this agent" rather than "is this agent's
composer".

## Risks / Trade-offs

- **The terminal grabs focus from something the user is using** → The latch is
  what prevents it: focus moves once per selection, never on a redraw. The
  composer has run on this rule since `acp-panel-ui` was written.
- **A session that never produces a grid never takes focus** → Correct, and
  visible: the pane says it is still starting. A session that fails is removed
  by the exit path, which clears the latch.
- **`has_live_grid` is a third place that has to agree with
  `render/content.rs`'s branch order** → Same exposure the existing facts
  carry, and the same mitigation: `tests/composer_focus.rs` is written against
  that order, and this change extends it rather than adding a second place to
  keep in step.
