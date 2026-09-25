# Design

## Context

See proposal.md - Why. Each numbered family's default modifier comes from one
function, `Shortcut::default_modifiers` (`crates/knot/src/keymap/shortcut.rs`).
Everything else derives from it:

- `Resolved::defaults` builds the default keymap from it.
- `Resolved::to_settings` stores only a modifier that differs from the
  default, so a user on the defaults has nothing stored.
- `Resolved::from_settings` applies stored values over the defaults one at a
  time, workspace family first, validating each against what is already
  accepted and skipping any that conflicts.
- The Keyboard tab, the View menu's select items (PR #477) and the sidebar
  hints (`sidebar-key-hints`, also in PR #477) label chords from the resolved
  keymap, not from literals.

## Goals / Non-Goals

**Goals:** swap the two defaults with no preference migration and no change
to validation, persistence or dispatch.

**Non-Goals:** any new setting (such as a "classic layout" toggle), and any
change to the order `from_settings` applies stored values in.

## Decisions

### Swap the defaults in `default_modifiers` only

The change is two match arms. Moving the Swift-reference comment to the
workspace arm's new value would misstate the reference; the comment moves to
explain the divergence instead (agents take plain ⌘ because they are switched
more often).

Alternative considered: a one-time migration that writes the old defaults
into existing users' preferences, so their chords do not move. Rejected: the
point of the change is that the new layout is better, and diff-only storage
exists precisely so that a changed default reaches everyone who never
customized it.

### Rely on diff-only storage for upgrades

Stored preferences written by `to_settings` cannot conflict with the new
defaults. A stored workspace modifier was never ⌘ (the old default), and a
stored agent modifier was never ⌥⌘ (the old default), so neither equals the
other family's new default. A user who customized one family keeps that
value; the other family takes its new default.

The one conflicting input is a hand-edited document that stores a default
explicitly, such as `workspaceSelectModifiers` = ⌘. `from_settings` validates
the workspace value against the default agent ⌘, rejects it, and keeps both
new defaults. That is the existing "stored binding that fails these rules is
ignored" behavior, and the spec delta adds a scenario for it rather than new
logic.

## Risks / Trade-offs

- [Existing users' muscle memory breaks on upgrade] → Intended. Note the swap
  in the release changelog through the `feat` commit. The Keyboard tab can
  restore the old layout, but because the families may never share a
  modifier it takes three steps: workspace to a free modifier such as ⌃⌘,
  agent to ⌥⌘, then workspace to ⌘.
- [⌘N inside the terminal or composer might be taken before the keymap sees
  it] → ⌘1–⌘9 was already a context-less binding (workspace selection) and
  worked from both, and no composer or terminal binding uses ⌘ with a digit.
  Confirm by pressing ⌘2 with the terminal and then the composer focused.
- [Swapping the defaults in one step could briefly make them collide] → Not
  possible: `Resolved::defaults` builds both families from the new values at
  once; validation only runs on customizations.

## Migration Plan

None. Rollback is reverting the commit; stored preferences are unaffected
either way.
