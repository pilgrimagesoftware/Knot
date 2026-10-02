# Proposal

## Why

`gpui-kit` 0.7.0 is out; Knot is on 0.6.4 plus a private fork
(`ADR-0002`) carrying two patch commits, and issue #565's fix added two
more to that same fork branch. Staying on 0.6.4 means every new fix piles
onto a fork that has to be hand-rebased forever. Moving to 0.7 lets one
of the two upstream commits retire (auto-grow's bound relax shipped
upstream) and gives a clean base to re-derive the rest against.

## What Changes

- Bump `gpui-kit`/`gpui-component`/`gpui-base` from 0.6.4 to 0.7.0.
- Re-derive the `gpui-base` fork off the `v0.7.0` tag as a new branch
  (`knot/auto-grow-0.7.0`): drop the auto-grow bound-relax commit (now
  upstream), keep the indent-on-newline-to-layout fix, re-apply issue
  #565's two Shift+Enter fixes against 0.7's `enter()`.
- Re-point `[patch.crates-io]` at the new branch.
- **BREAKING (internal only)**: delete `app_support::root_overlays()` and
  its 13 `.children(root_overlays(window, cx))` call sites -
  `Root::render_dialog_layer`/`render_sheet_layer`/`render_notification_layer`
  are removed in 0.7.0; `Root` now hosts all three layers itself
  automatically.
- Supersede `docs/adr/0002-gpui-base-auto-grow-fork.md` with a new ADR
  recording the re-derived fork and its own exit condition.
- Out of scope, follow-up only: migrating 28 `cx.open_window` call sites
  to the new `gpui_kit::open_window` helper (optional, old pattern still
  compiles). Plot and text-selection API changes: no usage in this repo,
  nothing to do.

## Capabilities

No user-visible behavior changes. Dialogs, sheets, notifications, and the
panel composer (auto-grow, indent-on-newline, Shift+Enter submit) all
behave identically before and after - this is a dependency and internal
API migration, not a feature change. `skip_specs: true` is set
accordingly.

## Impact

- `Cargo.toml` (`[patch.crates-io]` entry), `Cargo.lock`.
- `crates/knot/src/app_support.rs` (delete `root_overlays`) and its 13
  callers (every window's render function - see tasks.md).
- `docs/adr/0002-gpui-base-auto-grow-fork.md` (superseded).
- New fork branch `knot/auto-grow-0.7.0` on
  `pilgrimagesoftware/gpui-kit`, replacing `knot/auto-grow-0.6.4`.
- Full `make` gate (build, test, lint, fmt) must pass after the bump.
