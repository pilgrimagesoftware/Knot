# Proposal

## Why

Panel message timestamps (issue #577, PR #579) currently render beside every
prompt and response at all times. That is a constant, low-value label taking
up vertical space in every message — the same tradeoff the sidebar already
resolved for its own shortcut labels (`sidebar-key-hints`: hidden until ⌘ is
held, then shown as an overlay badge that takes no layout space). Timestamps
should follow the same convention: available on demand, invisible otherwise.

## What Changes

- Panel message timestamps (`relative_timestamp`/`absolute_timestamp` in
  `panel_view/message.rs`) are hidden by default and shown only while ⌘ is
  held, mirroring `workspace_window/key_hints.rs`'s hold-delay-then-show
  behavior (same delay constant, same clear-on-key-down/clear-on-deactivate
  rules).
- The panel view gains its own `KeyHintHold`-shaped state (or reuses the
  existing one if the panel and sidebar share a window) so a ⌘ hold while the
  panel has focus reveals every visible message's timestamp at once, not a
  single row's.
- Timestamps no longer reserve layout space (`v_flex` row above the message)
  when hidden — this was already their resting state but is now also the
  *default* state rather than the exception, so this is the common path.
- **BREAKING** (behavioral, not API): a user who expected the always-on
  timestamp from #577 now must hold ⌘ to see it.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
(none — `acp-panel-ui` has no existing timestamp requirement yet; #577/PR #579
has not landed on `develop`. This change adds the requirement directly as
intended behavior rather than modifying one that does not exist in the spec
tree yet. See `specs/acp-panel-ui/spec.md`'s ADDED Requirements.)

## Impact

- `crates/knot/src/panel_view/message.rs`: `render_timestamp` gated behind a
  held/not-held flag instead of rendering unconditionally.
- `crates/knot/src/panel_state/` or `workspace_window/`: new or reused hold
  tracking wired to `ModifiersChangedEvent`/`KeyDownEvent`/window-activation,
  following `workspace_window/key_hints.rs`.
- `crates/knot-core/consts.rs` (or `knot/src/consts.rs`): reuse
  `KEY_HINT_DELAY` rather than introduce a second delay constant.
- Depends on issue #577 / PR #579 landing on `develop` first — that PR is
  still open, so this change's worktree branches from it, not from `develop`.
