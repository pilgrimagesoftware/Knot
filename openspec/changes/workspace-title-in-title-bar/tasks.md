# Tasks

## 1. A testable seam for the title

- [x] 1.1 Add a pure resolver that maps a workspace id to the title to show —
      `Option<String>` from the store's workspaces, `None` when the id is not
      there — beside the other workspace-window helpers, not inside the render
      function; verify it compiles with `make build`
- [x] 1.2 Add `crates/knot/src/tests/workspace_title.rs` (registered in
      `tests/mod.rs`) covering: the named workspace resolves to its name, a
      renamed workspace resolves to the new name from the same store, an unknown
      id resolves to `None`, and a name carrying a newline resolves to one line
      via `single_line`; verify with `cargo test -p knot workspace_title`

## 2. The sidebar title bar

- [ ] 2.1 Turn `WorkspaceWindow::sidebar_title_bar` into a method and have it
      render the application icon plus the resolved workspace name in place of
      `l10n::t("app.name")`, taking the name from the same store lock scope the
      frame already holds for the agent rows; verify by running the app
      (`make build` then launch) with a workspace named something other than
      "Knot" and reading its title bar
- [ ] 2.2 Update the doc comment on `sidebar_title_bar` — it currently explains
      why compact drops the *application name* — and give the compact branch the
      workspace name as a tooltip, matching the compact agent row
      (`render/sidebar.rs:73`); verify by dragging the sidebar below 160px and
      hovering the title bar
- [ ] 2.3 Confirm a long workspace name truncates on one line and does not
      displace the traffic lights, applying `min_w_0()` on the label's flex child
      if it does not — `flex_1()` alone will not shrink it; verify by renaming a
      workspace to a ~120-character name and narrowing the window

## 3. Keeping the title current

- [x] 3.1 Add a `titled_as: String` field to `WorkspaceWindow`, initialised from
      the name `open.rs` already sets as the OS title, with a comment saying it
      caches the last value *written* to AppKit and is not a copy of the state
      (per design.md, and issue #238 for the failure it avoids); verify with
      `make lint`
- [ ] 3.2 In `render`, call `window.set_window_title` when the resolved name
      differs from `titled_as`, then store it; verify by renaming a workspace
      with its window open and checking the Window menu and Mission Control show
      the new name
- [ ] 3.3 Call `cx.refresh_windows()` after a successful `rename_workspace` in
      `workspace_manager::save_name`, with a comment pointing at why `cx.notify()`
      is not enough (`import_window/window.rs:164` is the precedent); verify by
      opening two workspace windows, renaming one, and watching only that
      window's title bar change

## 4. Gate

- [x] 4.1 Run `make` and confirm the whole gate passes — `fmt-check`,
      `size-check`, `clippy -D warnings`, tests, build
- [ ] 4.2 Walk the spec's scenarios against the running app: two open windows
      name themselves differently, a rename reaches the open window and its OS
      title, other windows are untouched, a long name ellipsizes, and the
      compact sidebar offers the name as a tooltip
