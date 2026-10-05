---
id: ADR-0003
title: Carry the gpui-base fork onto gpui-kit 0.7.0
status: accepted
date: 2026-10-05
tags: [ui, dependencies, gpui-kit]
supersedes: ADR-0002
---

# ADR-0003: Carry the gpui-base fork onto gpui-kit 0.7.0

## Context

ADR-0002 forked `gpui-base` off `v0.6.4` so the panel composer could be an
`EditorState` with auto-grow. Its exit condition was an upstream release that
relaxes the `TextareaMode`-only bound on `auto_grow`, `set_auto_grow`, `rows`
and `set_rows`.

Since then the patch branch `knot/auto-grow-0.6.4` has picked up two more
commits, both for issue #565: a configured Shift+Enter submit chord must not
also insert a newline, and a multi-line submit chord must not propagate
`Enter` on to a second handler that types one.

Knot now moves to gpui-kit 0.7.0 (issue #575), the release Fernrohr pins. A
terminal view shared between the two apps only works if both resolve the
same `gpui`, so the version is not ours to choose.

Checked against the published `gpui-base` 0.7.0, whose `src/` is identical to
the `v0.7.0` tag:

- The four row-sizing methods are still on `impl InputBaseState<TextareaMode>`
  (`input/base/state.rs`). **ADR-0002's exit condition is not met.**
- `enter()` still decides indent-on-newline from `is_code_editor()`, which
  answers from the mode marker, so an `EditorState` composer would still
  inherit the previous line's indent.
- The #565 lines in `enter()` are unchanged from 0.6.x.

## Decision

We keep consuming a forked `gpui-base` through `[patch.crates-io]`, now from
branch `knot/auto-grow-0.7.0` on `pilgrimagesoftware/gpui-kit`. It is cut from
tag `v0.7.0` and carries the same four commits as `knot/auto-grow-0.6.4`,
cherry-picked with `-x`:

1. Allow auto-grow and row sizing on any multi-line mode.
2. Key indent-on-newline to the layout, not the mode marker.
3. Stop Shift+Enter inserting a newline when it is the submit chord (#565).
4. Don't propagate Enter past a multi-line submit chord (#565).

All four applied without conflicts. The diff against `v0.7.0` is two files
(`input/base/mode.rs`, `input/base/state.rs`), +82/-21, and the fork's own
`cargo test -p gpui-base` passes all 1226 tests.

`crates/knot` pins `gpui-kit = "=0.7.0"` and the lockfile pins
`gpui-component-macros` to 0.7.0, so Knot's set of `gpui*` crates matches
Fernrohr's lockfile exactly.

## Options considered

- **Cherry-pick the four commits onto `v0.7.0`** (chosen). The commits are
  small and applied cleanly, so a reviewer only has to check that the diff
  matches the 0.6.4 one.
- **Drop the bound-relax commit as already upstream.** The change's design
  assumed this, but 0.7.0 does not carry it. Dropping it would leave
  `EditorState` with no `auto_grow` and break the composer's build.
- **Re-derive the #565 fixes by hand.** The design expected `enter()` to have
  moved too far for a cherry-pick. It had not, and rewriting known-good
  changes would only have added risk.
- **Stay on 0.6.x.** Rules out sharing the terminal view with Fernrohr, which
  is the reason for the bump.

## Consequences

- Every point in ADR-0002's Consequences still holds for the new branch:
  `cargo tree` must show one `gpui-base`, the patched one, and deleting or
  force-pushing the branch breaks CI.
- `knot/auto-grow-0.6.4` is kept so that reverting this change still builds.
  Delete it once 0.7 has shipped in a Knot release.
- Bumping gpui-kit is now a change to two repos: Fernrohr's pin and Knot's
  pin move together, and this fork is rebased with them.

### Exit condition

When a published `gpui-base` release has the relaxed row-sizing bound, keys
indent-on-newline to the layout, and handles the submit chord the #565 way:

1. Delete the `[patch.crates-io]` entry from the root `Cargo.toml`.
2. Delete the `knot/auto-grow-*` and `relax-auto-grow-bound` branches on
   `pilgrimagesoftware/gpui-kit`.
3. Verify that `make` passes and that `cargo tree` shows the registry
   `gpui-base`.
4. Supersede this record.

If a release carries only some of these, re-cut the fork from that release
with just the remaining commits, and record that in a new ADR.
