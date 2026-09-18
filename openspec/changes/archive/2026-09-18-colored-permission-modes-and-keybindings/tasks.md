## 1. Risk-level classification

- [x] 1.1 Add a `RiskLevel` enum (`Danger`/`Safe`/`Neutral`) and a
      `permission_risk_level(value: &str) -> RiskLevel` keyword-matching
      function; verify with unit tests covering `bypassPermissions`,
      `plan`, and an unrecognized value each map to the expected level,
      case-insensitively.

## 2. Permission-mode selector coloring

- [x] 2.1 At the `find_config_option` call site for the permission-mode
      selector in `main.rs`, compute the `RiskLevel` for the current value
      and for each dropdown entry's value; verify by reading the updated
      call site and confirming both the button and each `PopupMenuItem`
      receive a risk-derived color.
- [x] 2.2 Apply the corresponding color to the selector button and each
      dropdown item without changing `render_panel_config_selector`'s
      behavior for the model/effort selectors; verify by running the app,
      switching an agent through its declared permission modes, and
      confirming button/dropdown colors match §1's classification while
      the model/effort selectors are unaffected.

## 3. Inline permission prompt coloring

- [x] 3.1 Thread the resolved permission-mode `RiskLevel` (or `Neutral`
      when undeclared) from `render_panel_pane` through
      `panel_view::render_panel` into `render_permission_prompt`; verify
      the function signatures compile and existing panel tests still pass.
- [x] 3.2 Apply the risk color to the prompt's border, falling back to the
      current neutral blue when no permission-mode option is declared;
      verify by manually triggering a permission request against an agent
      in a bypass-like mode and confirming the border renders in the
      danger color, then against one with no declared mode and confirming
      the existing neutral border is unchanged.

## 4. Keybindings

- [x] 4.1 Survey `crates/knot/src/main.rs`'s existing `bind_keys` calls and
      the terminal/textarea key-forwarding path to choose collision-free
      key combinations for allow, deny, and open-permission-selector;
      verify by documenting the chosen combinations in this task's commit
      message.
- [x] 4.2 Add `PanelPermissionAllow`, `PanelPermissionDeny`, and
      `PanelOpenPermissionSelector` to the `actions!(knot_app, ...)` block
      and register their key bindings; verify the crate builds with
      `make rust-build`.
- [x] 4.3 Wire `PanelPermissionAllow`/`PanelPermissionDeny` handlers to
      the same allow/deny path as the existing `on_decision` click handler,
      resolving the focused/relevant panel's `pending_permission`; verify
      with a test (or manual run) that invoking the binding with a pending
      request resolves it, and invoking it with none pending is a no-op.
- [x] 4.4 Wire `PanelOpenPermissionSelector` to open the permission-mode
      selector's dropdown when the panel input area has focus; verify
      manually that the binding opens the same dropdown a click would.

## 5. Verification

- [x] 5.1 Run `make rust` (fmt + clippy + test + build) and confirm it
      passes with no new warnings.
- [ ] 5.2 Manually exercise the full flow against a real ACP agent
      connection: switch permission modes via mouse and keyboard, trigger a
      permission prompt, resolve it via mouse and keyboard, and confirm
      colors and keybindings behave per `specs/permission-prompt-ui/spec.md`.
