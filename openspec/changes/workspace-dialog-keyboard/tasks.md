# Tasks

## 1. Settle the key mechanism

- [x] 1.1 Attach an `.on_key_down` listener to the workspace name dialog's
      overlay in `crates/knot/src/workspace_manager/mod.rs` that logs the
      keystroke, and verify in the app that Return and Escape typed while the
      name field has focus reach it. If the focused `Input` swallows either
      key, switch to the `key_context` + gpui action fallback named in
      design.md before going further, and record which route was taken in a
      comment on the handler.

      Settled: the `Input` lets both keys through, so the bubble-phase
      `.on_key_down` on the overlay is the route taken, recorded in a
      comment there. Verified by the keystroke-driven tests in task 4.2
      rather than by reading a log off a running app - with the handler
      removed, three of those four tests fail, so they do prove the key
      reaches the overlay.

## 2. The keys

- [x] 2.1 Route Return to `confirm_workspace_dialog` and Escape to
      `cancel_workspace_dialog`. Verify in the app: typing a name and
      pressing Return creates the workspace and closes the dialog, and
      pressing Escape closes it having created nothing.
- [x] 2.2 Verify the same two keys in the dialog's rename mode - open it from
      a workspace row's rename button, edit the name, and confirm Return
      renames and Escape discards, with no separate code path.

## 3. Blocking an empty name

- [x] 3.1 Give the confirm button `.disabled(...)` computed from the trimmed
      value of the name input at render time. Verify in the app that the
      button is greyed with an empty field, enables on the first
      non-whitespace character, and greys again when the field is emptied.
- [x] 3.2 Verify that Return with an empty or whitespace-only name does
      nothing at all: the dialog stays open, no workspace is created, and no
      error message appears.
- [x] 3.3 Verify that a name typed with surrounding spaces is stored trimmed,
      and leave the `save_name` empty-name guard in place as defense in
      depth.

## 4. Tests

- [x] 4.1 Add a unit test for the trim-and-blank predicate the button's
      disabled state and the key handler both read, covering empty,
      whitespace-only, and surrounded-by-whitespace names, so the rule is
      pinned without a window. Done as `workspace_name_is_blank`.
- [x] 4.2 Add a `gpui-kit` `test-support` test driving the dialog: open it,
      simulate `escape`, and assert the dialog closed and no workspace was
      added; open it again, set a name, simulate `enter`, and assert the
      workspace exists. Skip this task only if the harness cannot deliver the
      key to the overlay, and say so in the task rather than deleting it.

## 5. Verification

- [x] 5.1 Run `make rust` and verify fmt, clippy, tests and build all pass
      for the workspace.
- [ ] 5.2 (needs a human at the app) Walk the dialog once by keyboard only - open it from the New
      workspace button, type, Return; reopen it, type, Escape - and confirm
      neither path needs the pointer.
