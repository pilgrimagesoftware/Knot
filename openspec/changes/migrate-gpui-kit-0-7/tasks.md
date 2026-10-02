# Tasks

## 1. Re-derive the gpui-base fork

- [ ] 1.1 Cut `knot/auto-grow-0.7.0` from `pilgrimagesoftware/gpui-kit`'s `v0.7.0` tag; verify `git log -1` shows `v0.7.0` as the branch point
- [ ] 1.2 Re-apply the indent-on-newline-to-layout fix (ADR-0002 commit 2); verify `cargo test -p gpui-base` passes, including the indent-inheritance test ADR-0002 names
- [ ] 1.3 Re-derive issue #565's submit-chord-newline fix against 0.7's `enter()`; verify the ported `test_shift_enter_submits_without_inserting_newline_when_configured_as_submit_chord` test passes
- [ ] 1.4 Re-derive issue #565's propagate fix against 0.7's `enter()`; verify `cargo test -p gpui-base input::` passes in full (273+ tests, zero failures)
- [ ] 1.5 Push the branch and open a PR against it describing both re-derived fixes; merge once green

## 2. Bump the pin in Knot

- [ ] 2.1 Update `Cargo.toml`'s `[patch.crates-io]` entry to `knot/auto-grow-0.7.0`; update `gpui-kit`/`gpui-component` version requirements to 0.7; verify `cargo update -p gpui-base --precise <merged-sha>` and `cargo build --workspace` succeed
- [ ] 2.2 Delete `app_support::root_overlays` and its 13 `.children(root_overlays(window, cx))` call sites (grep `root_overlays` across `crates/knot/src` for the full list); verify `cargo build --workspace` compiles with no dangling reference
- [ ] 2.3 Run `cargo test --workspace`; fix any test that encoded old-fork or pre-0.7 behavior as expected (same shape as the `panel_composer` test issue #565 touched)

## 3. Close out

- [ ] 3.1 Supersede `docs/adr/0002-gpui-base-auto-grow-fork.md` with a new ADR recording the 0.7.0-based fork, its single remaining commit, and its own exit condition; verify `docs/adr/README.md`'s index is updated
- [ ] 3.2 Run the full `make` gate (fmt-check, size-check, lint, test, build); verify all pass
- [ ] 3.3 Manually smoke-test a dialog, a sheet, a notification, and the panel composer's Shift+Enter submit in a running build; verify no stray newline and no missing overlay layer
- [ ] 3.4 Open the Knot PR referencing issues #565 and #575; verify CI is green before requesting merge
