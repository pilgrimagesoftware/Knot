# Tasks

## 1. Window registry

- [x] 1.1 Add `crates/knot/src/window_registry.rs` with `WindowKey`
      (`Workspace(Uuid) | CommandCenter | WorkspaceManager`) and a
      `WindowRegistry` global holding `HashMap<WindowKey, AnyWindowHandle>`;
      verify `cargo build --workspace` succeeds and `make size-check` passes
- [x] 1.2 Implement `activate(key, cx) -> bool` using the
      `handle.update(..).is_ok()` probe from `about_window/window.rs:46-61`,
      removing the entry when the probe fails; verify a unit test that seeds a
      stale key and asserts it is gone after a miss
- [x] 1.3 Implement `activate_or_open(key, cx, open_fn)` returning whether it
      activated or opened, and registering the handle on the open path; verify
      by building and by the call-site tasks below compiling against it
- [x] 1.4 Install the registry as a `cx.global` during boot in
      `app_bootstrap.rs`; verify the app launches and a workspace still opens

## 2. One window per workspace

- [x] 2.1 Route `WorkspaceWindow::open` and `open_with_selection`
      (`workspace_window/open.rs`) through `activate_or_open` with
      `WindowKey::Workspace(id)`; verify opening the same workspace twice from
      the manager leaves one window, raised
- [x] 2.2 Extract the "select this agent" step from `open_with_selection` into a
      method on `WorkspaceWindow` that both the fresh-open and the activate
      paths call; verify `make lint` passes with no duplicated selection logic
- [x] 2.3 On the activate path, downcast to `WindowHandle<WorkspaceWindow>` and
      apply that selection method; verify clicking a Command Center card for an
      agent in an already-open workspace raises that window with the agent
      selected
- [x] 2.4 All four call sites already funnel through `open_with_selection`, so
      the registry check in 2.1 covers them with no call-site edit; verified no
      other `cx.open_window` for a workspace window exists
      (`grep -rn "open_window" crates/knot/src/workspace_window/`)
- [x] 2.5 Confirm the registry entry is dropped when a workspace window is
      closed, so reopening that workspace opens a window; verify by closing and
      reopening a workspace in the running app

## 3. Command Center and manager as single windows

- [x] 3.1 Route `CommandCenterWindow::open` (`command_center.rs:44-67`) through
      `activate_or_open` with `WindowKey::CommandCenter`; verify clicking the
      manager's toolbar button twice leaves one Command Center window
- [x] 3.2 Route the workspace manager's open path (`app_bootstrap.rs:340-360`)
      through `WindowKey::WorkspaceManager`; verify the manager is registered at
      boot so the new menu item can raise it

## 4. Bounds reconciliation

- [x] 4.1 Add the title-bar strip height and minimum horizontal overlap
      constants to `crates/knot/src/consts.rs`; verify `make lint` passes
- [x] 4.2 Implement `reconcile_bounds(saved, displays) -> Option<Bounds<Pixels>>`
      in `window_options.rs` (or a sibling if the file nears 700 lines) per
      design.md - contained, too-large, overlapping, disjoint; verify unit tests
      covering all four branches plus the "greatest overlap wins" tie
- [x] 4.3 Call it from `workspace_window_options` with `cx.displays()`, falling
      back to the centered default on `None`; verify a workspace whose saved
      bounds lie at x=4000 opens on the attached display with its title bar
      visible
- [x] 4.4 Guarded the write-back: `cx.observe_window_bounds` only fires from
      the platform resize callback, so it does not see the initial placement,
      but the observer now also skips a notification whose bounds equal what we
      placed - a spurious post-creation resize would otherwise overwrite the
      remembered frame with the fallback. Manual check (reconnect the display,
      confirm the window returns to its remembered position) is in 7.2

## 5. Menu bar

- [x] 5.1 Remove the `EnterFullScreen` item from the View menu in
      `set_app_menus` (`app_bootstrap.rs:251-252`), leaving that menu with no
      items of its own, and remove the name from `actions!` (`:129-134`), the
      `cmd-ctrl-f` binding (`:310`) and the assertion at
      `crates/knot/src/tests/menu_key_equivalents.rs:92`; verify `make test`
      passes
- [ ] 5.2 Verify the View menu shows exactly one Enter Full Screen item, that
      it is enabled, that ⌃⌘F enters and leaves full screen, and that the item
      reads Exit Full Screen while full screen - including on the settings
      window and after switching away from Knot and back
- [x] 5.3 Add `OpenCommandCenter` and `OpenWorkspaces` to `actions!`, register
      their `cx.on_action` handlers calling the registry, and bind `cmd-alt-0`
      and `cmd-0`; verify both shortcuts raise the right window with every Knot
      window closed and with several open
- [x] 5.4 Add `menu.window.command_center` and `menu.window.workspaces` to
      `crates/knot-core/locales/en.yml` and use `l10n::t` for both item labels;
      verify the l10n key test resolves them, touching `knot-core` first so the
      build is not stale
- [x] 5.5 Declare the Window menu as Command Center, Workspaces, separator,
      Minimize, Zoom, separator; verify the menu shows that order with AppKit's
      window list below the trailing separator, and that the fixed items stay
      put after opening three more windows
- [x] 5.6 Extend `menu_key_equivalents.rs` with assertions for `cmd-alt-0` and
      `cmd-0`; verify `make test` passes

## 6. Command Center scrolling

- [x] 6.1 Wrap the Command Center's workspace sections in a vertical scroll
      container in `command_center.rs`'s render; verify a Command Center with
      enough agents to overflow scrolls to the last card and that card is
      clickable
- [ ] 6.2 Confirm a short grid neither scrolls nor reserves scrollbar space;
      verify with a single-workspace, two-agent Command Center

## 7. Gate

- [x] 7.1 Run `make` and verify the full gate passes - fmt, size-check, lint,
      test, build
- [ ] 7.2 Walk the scenarios in `specs/window-lifecycle/spec.md`,
      `specs/app-menu/spec.md` and `specs/dashboard/spec.md` against the running
      app and verify each holds
