# Design

## Context

See proposal.md - Why. Relevant state:

- `ADR-0002` forks `gpui-base` off tag `v0.6.4` (branch
  `knot/auto-grow-0.6.4`), carrying two commits: relax the auto-grow bound
  off `TextareaMode` onto `impl<M: MultiLineMode>`, and key
  indent-on-newline to `LayoutMode` instead of the mode marker.
- Issue #565 added two more commits to that same branch: stop
  `insert_newline` from force-inserting on a configured submit chord, and
  stop `cx.propagate()` from re-dispatching past a multi-line submit
  chord.
- Checked directly against `gpui-base-0.7.0` from crates.io: the
  auto-grow bound relax is upstream now (exit condition met for that one
  commit); the indent-layout fix is not; #565's two fixes are fork-only
  and 0.7's `enter()` has moved enough that they need re-deriving, not a
  cherry-pick.
- 0.7.0's only breaking changes that touch this repo: `Root::render_*_layer`
  removal (13 call sites via one helper) and the Plot API (no usage, no
  action).

## Goals / Non-Goals

**Goals:**
- Land on 0.7.0 with the same patch behavior Knot already has (auto-grow,
  indent-on-newline, Shift+Enter submit with no stray newline), nothing
  more and nothing less.
- Shrink the fork to the one commit that's still needed, now that the
  bound relax is upstream.

**Non-Goals:**
- Migrating `cx.open_window` call sites to `gpui_kit::open_window`
  (optional cleanup, separate change).
- Any other 0.7 feature adoption (Toolbar, Questionnaire, TimeField,
  Plot).

## Decisions

**New fork branch, not a rebase of the old one.** `knot/auto-grow-0.6.4`
is cut from `v0.6.4`; its auto-grow commit no longer applies cleanly
against 0.7.0 (upstream already carries an equivalent change, so a rebase
would conflict for no reason - the commit is just dropped). Cut
`knot/auto-grow-0.7.0` from `v0.7.0` fresh, re-apply only the indent-layout
fix, then re-derive #565's two fixes against 0.7's `enter()` shape rather
than cherry-picking - the function has moved enough (confirmed by reading
both versions) that a cherry-pick would conflict line-by-line anyway.
Verify each re-derived fix against the same tests that caught the
original bugs (`gpui-base`'s own `input::state::tests`, and Knot's
`panel_composer::each_chord_reports_and_types_the_same_either_way_round`).

**Delete `root_overlays` outright, don't shim it.** 0.7.0 gives no
replacement for the three removed `render_*_layer` methods because none
is needed - `Root` hosts them automatically now. A compatibility shim
would carry dead weight with no call site that still needs it; every one
of the 13 call sites is deleted in the same change, mechanically (drop
the `.children(root_overlays(window, cx))` line and, where it's the last
thing a `v_flex()`/`div()` chained, drop nothing else - the layer was
additive, not load-bearing for layout).

**Supersede ADR-0002 rather than edit it in place.** ADR-0002 is a closed
record of a decision made at a point in time (fork off 0.6.4, two
commits, exit condition tied to the auto-grow bound alone). The
exit condition was never fully met for that ADR as written - the
indent-layout commit was always going to outlive it - so a new ADR
records the actual state after this change: one fork commit, cut from
0.7.0, with issue #565's two fixes now living there too and their own
exit condition (an upstream release carrying both the indent-layout
keying and the Shift+Enter fixes).

## Risks / Trade-offs

[Re-deriving #565's fixes against 0.7's `enter()` introduces a new
defect, since it's new code against unfamiliar surrounding logic rather
than a mechanical port] → Re-run both regression tests that caught the
originals (gpui-base's own test plus Knot's `panel_composer` test) against
the 0.7-based fork before bumping the pin, not after.

[Deleting `root_overlays` call sites by hand across 13 files misses one,
leaving a dangling `.children(root_overlays(...))` that no longer
compiles - or worse, compiles if an `#[allow(dead_code)]` masks it] →
`cargo build --workspace` will not compile if any call site still
references the deleted function; no masking is possible since it's not a
warning-suppressible case, it's a missing-item compile error.

[The fork branch disappears or force-pushes, breaking CI for everyone
building `develop`] → Same risk ADR-0002 already accepted for
`knot/auto-grow-0.6.4`; no new mitigation beyond what's already in place
(the branch lives under `pilgrimagesoftware/gpui-kit`, same org).

## Migration Plan

1. Cut `knot/auto-grow-0.7.0` from `pilgrimagesoftware/gpui-kit`'s
   `v0.7.0` tag.
2. Re-apply the indent-layout fix; re-derive #565's two fixes against
   0.7's `enter()`.
3. Run `gpui-base`'s own `input::` test module and push for review.
4. Once merged, bump `Cargo.toml`'s `[patch.crates-io]` entry and
   `gpui-kit`/`gpui-component` versions to 0.7.0 in Knot; `cargo update`
   the lockfile.
5. Delete `app_support::root_overlays` and its 13 call sites.
6. Supersede ADR-0002 with a new ADR.
7. Full `make` gate; manual smoke of a dialog, a sheet, a notification,
   and the panel composer's Shift+Enter submit.

Rollback: revert the Knot-side commit(s); the old fork branch
(`knot/auto-grow-0.6.4`) stays intact and unaffected, since this plan
never deletes it.
