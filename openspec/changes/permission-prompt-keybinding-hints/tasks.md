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
- [ ] 3.2 Trigger a permission prompt against a live ACP agent and confirm
      the buttons read "Allow ⇧⌘A" and "Deny ⇧⌘D", that pressing each
      resolves the request, and that clicking still works.
