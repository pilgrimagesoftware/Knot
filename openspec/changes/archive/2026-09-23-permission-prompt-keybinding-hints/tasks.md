## 1. Show the binding on the decision buttons

- [x] 1.1 Thread the frame's `Window` from `render_panel`'s list closure
      through `render_row` into `render_permission_prompt`; verify the
      crate builds with `make build`.
- [x] 1.2 Look the allow and deny keystrokes up with
      `Kbd::global_binding_for_action` and render each as a child of its
      own button, so an unbound action contributes no element; verify by
      reading the call site and confirming the `Option<Kbd>` is passed
      through `children`, which draws nothing for `None`.

## 2. Guard the bindings

- [x] 2.1 Add a test asserting `cmd-shift-a` and `cmd-shift-d` resolve to
      `PanelPermissionAllow` and `PanelPermissionDeny` over the real
      keymap, so removing a binding fails the build rather than silently
      dropping the hint; verify with `make test`.

## 3. Verification

- [x] 3.1 Run `make` and confirm fmt, size-check, clippy, test and build
      all pass.
- [x] 3.2 Assert the hint reaches the painted frame rather than merely
      resolving from the keymap: render the prompt in a real window and
      look each keystroke up in `debug_bounds`. Verified by deleting the
      `children(allow_kbd)` call and confirming the test fails, so it
      cannot pass for the wrong reason.
- [ ] 3.3 Still manual, and not done: trigger a prompt against a live ACP
      agent and confirm the hints read "Allow ⇧⌘A" / "Deny ⇧⌘D" at a
      glance, that pressing each key resolves the request, and that
      clicking still works. 3.2 proves the element is painted; it proves
      nothing about legibility against the primary button fill, and it
      exercises neither the key dispatch nor the click path end to end.

      Carried forward, not dropped: the change is archived with 3.3 still
      open. The risk-colouring half of #194 has to trigger a live
      permission prompt to verify its own fix, so this check moves into
      that change rather than waiting on an occasion of its own.
