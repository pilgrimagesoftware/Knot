---
id: ADR-0002
title: Fork gpui-base to allow auto-grow on any multi-line mode
status: accepted
date: 2026-09-23
tags: [ui, dependencies, gpui-kit]
---

# ADR-0002: Fork gpui-base to allow auto-grow on any multi-line mode

## Context

The panel composer styles what the user is typing - slash tokens, `@`
references, markdown, attachment chips
(`openspec/changes/rich-prompt-composer`). Styled ranges reach the screen
through one of two seams in `gpui-base`, and both are `EditorState`-only:

- `InputHighlighter` is stored *inside* the `LayoutMode::CodeEditor` variant,
  so reaching it means adopting code-editor layout - the one thing
  `panel-rich-input`'s "The composer stays a composer" forbids.
- `TextDecorationCollection` is stored in `state.extras`, keyed off the **mode
  marker**. `InputModeKind::Extras` is `()` for both `InputMode` and
  `TextareaMode`, and `impl InputExtras for ()` takes the default
  `decoration_layers()`, which returns an empty `Vec`. A textarea cannot carry
  a decoration, ever.

So the composer needs `EditorState`. It also needs the auto-grow sizing it
already has: it grows with its content between a collapsed and an expanded
bound, re-issued by the expand control.

It cannot have both. `auto_grow`, `set_auto_grow`, `rows` and `set_rows` all
sit in `impl InputBaseState<TextareaMode>` (`state.rs:9158` in 0.6.4);
`EditorState` has no row sizing at all.

The bound is the only obstacle. The element's auto-grow machinery is already
generic over the mode marker: `element.rs` reads `state.mode.is_auto_grow()`,
`.rows()` and `.max_rows()` from `impl<M: InputModeKind>` code, and
`LayoutMode::update_auto_grow` sizes from `display_map.wrap_row_count()`
without consulting the marker. **Nothing in the layout or render path requires
a textarea to auto-grow.** 0.6.6, the latest release, is identical on every
point above.

## Decision

We consume a forked `gpui-base` through `[patch.crates-io]` in the root
`Cargo.toml` until upstream releases the relaxed bound.

The fork moves those four methods into the **existing**
`impl<M: crate::input::MultiLineMode> InputBaseState<M>` block, with no change
to any body. `TextareaMode` keeps `new()`, which is genuinely mode-specific.
Git reads the change as 11 insertions and 11 deletions in one file.

Two branches on `pilgrimagesoftware/gpui-kit`, carrying the same commit:

| Branch | Base | Purpose |
| --- | --- | --- |
| `knot/auto-grow-0.6.4` | tag `v0.6.4` | what `[patch.crates-io]` points at |
| `relax-auto-grow-bound` | `main` | what the upstream PR is opened from |

The patch branch is cut from `v0.6.4` - the version the workspace already
resolves - so the patch is those four methods and nothing else. Cutting it
from `main` would have shipped every change since 0.6.4 as well: a version
bump wearing a patch's clothes, and a much larger thing to re-verify.

Upstream PR: **not yet opened**. The branch is pushed and the description is
ready; opening it against `longbridge/gpui-kit` is a maintainer action and has
been left to the repository owner. Record the PR URL here once it exists.

## Options considered

- **Fork and patch, upstream in parallel** (chosen) - the composer ships now,
  and the fork has a defined end. The diff is four method bodies moved between
  two impl blocks that already exist, which is small enough to re-verify
  mechanically on every gpui-kit bump.
- **Reimplement auto-grow in Knot** - means reproducing
  `display_map.wrap_row_count()`, which is crate-private. Our height would be
  an independent guess at the element's own wrapping and would drift on any
  change to it.
- **Drop to two fixed heights** - a user-visible regression in a control the
  user touches constantly, to avoid a dependency problem the user cannot see.
- **Wait for an upstream release** - puts a shipped feature on a third party's
  schedule, with no commitment behind it.
- **Vendor `gpui-base` into the repo** - works for CI, but copies a whole
  third-party crate into the tree and still needs a fork for the PR.

## Consequences

- `gpui-base` is a dependency of both `gpui-component` and `gpui-kit`, so the
  patch applies workspace-wide. `cargo tree` must continue to show exactly one
  `gpui-base`, the patched one.
- Every gpui-kit bump has to rebase this. That is the point of keeping it to
  four moved methods: a bump that will not rebase mechanically means the
  upstream shape changed, which is exactly when a human should look.
- CI fetches the fork as a git dependency. A branch deleted or force-pushed on
  `pilgrimagesoftware/gpui-kit` breaks the build for everyone.
- The fork's own suite passes (975 tests on the patch branch, 1078 on the PR
  branch), so the move is not carrying a behaviour change with it.

### Exit condition

When a published `gpui-base` release carries the relaxed bound:

1. Delete the `[patch.crates-io]` entry from the root `Cargo.toml`.
2. Delete both branches on `pilgrimagesoftware/gpui-kit`.
3. Verify `make` passes and `cargo tree` shows the registry `gpui-base`.
4. Supersede this record.

Until then, `Cargo.toml`'s comment above the patch entry points here, and
removing the fork is a task in the change's `tasks.md` rather than a
someday-item.
