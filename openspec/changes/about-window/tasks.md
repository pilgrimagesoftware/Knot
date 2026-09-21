# Tasks

## 1. Build stamping

- [x] 1.1 Add `crates/knot/build.rs` emitting `KNOT_BUILD_COMMIT` (short git
      SHA, `-dirty` suffix when the tree is dirty, `unknown` when `git` is
      unavailable or fails) and `KNOT_BUILD_DATE` (UTC `YYYY-MM-DD`), with
      `rerun-if-changed` on `.git/HEAD` and the ref it points at; add `time`
      as a `[build-dependencies]` entry from the workspace. Verified:
      `cargo build -p knot` succeeds and the build-script output carries
      `KNOT_BUILD_COMMIT=48381d13cd99-dirty` / `KNOT_BUILD_DATE=2026-09-21`.
- [x] 1.2 Verify the tarball path: built with a `git` on `PATH` that exits
      non-zero, `cargo build -p knot` still succeeded and stamped
      `KNOT_BUILD_COMMIT=unknown`.

## 2. About window content

- [x] 2.1 Add the `about:` strings to `crates/knot-core/locales/en.yml` -
      window title, version label, build label, copyright, and one credits
      key per attributed item (author, app license, bundled fonts, UI
      toolkit, terminal engine). Verified by
      `tests::about_window_labels_resolve`, which fails if any key falls back
      to the key string. Third-party licenses checked against each crate's
      own manifest rather than from memory.
- [x] 2.2 Add `crates/knot/src/about_window/mod.rs` with the build-identity
      values (version from `CARGO_PKG_VERSION`, build from the two stamped
      env values) exposed as functions returning the display strings, with
      unit tests over the version, a known commit, an unstamped commit, and
      this binary's own build identifier.
- [x] 2.3 Implement the `AboutWindow` view: icon (embedded
      `assets/icon/icon.png`, rendered at 128px), app name, version and
      build lines, copyright line, credits block, in that order.
- [x] 2.4 Add the copy affordance on the version/build line writing
      `"Knot <version> (<build>)"` to the clipboard, with a unit test over
      the formatting function.
- [x] 2.5 Add the non-macOS Close button calling `window.remove_window()`.
      Written with `cfg!` rather than `#[cfg]` so both arms compile on every
      platform; covered by `make rust-lint` (`-D warnings`) on macOS.

## 3. Opening and dismissing the window

- [x] 3.1 Add `about_window_options` to `window_options.rs`: fixed bounds,
      `is_resizable: false`, `is_minimizable: false`. The titlebar carries
      the catalog title only off macOS - on macOS it is untitled, matching
      the system About panel and the one-title-per-window rule in
      `.claude/rules/knot-ui-conventions.md`.
- [x] 3.2 Replace `about_knot`'s alert-dialog body with
      `open_about_window(handle, cx)`: raise the existing window when its
      update succeeds, otherwise open a new one and store the handle. The
      handle lives inside `register_about_action`, which `run` and the tests
      both call.
- [x] 3.3 Rewrite `crates/knot/src/tests/about_dialog.rs` as
      `about_window.rs`: dispatching `AboutKnot` opens exactly one window;
      a second dispatch opens no second window; a dispatch after the window
      is removed opens one again. `cargo test -p knot` passes, `tests/mod.rs`
      updated.
- [x] 3.4 Handle `Escape`: track a `FocusHandle` on the view root, focus it
      on first render, and close the window from an `on_key_down` listener
      matching the `escape` keystroke. Verified against the running app (see
      4.2), not in the headless tests.
- [x] 3.5 Update the `root_overlays` doc comment in `app_support.rs`, which
      named About as one of the dialogs opened through the dialog layer.

## 4. Verification

- [x] 4.1 `make rust` passes clean (fmt, clippy, tests, build).
- [x] 4.2 macOS verification, driven against the running app through the
      accessibility API: (a) About Knot opens the window and the Workspaces
      window raises in front of it while it stays open, so it is not modal;
      (b) choosing About again leaves one About window; (c) closing it and
      choosing About again reopens it; (d) `Escape` with the app frontmost
      closes it, as does the close button; (e) the window reports 360x592
      and refuses a resize, and its minimize button is disabled; (f) the
      version and build come from the crate version and the build stamp,
      covered by the unit tests in 2.2.
      Not verified this way: the rendered layout and the copy action - gpui
      draws its content into a single accessibility group, and screen capture
      is not permitted in this environment. The copied string is unit-tested;
      the layout needs an eye on it.
- [x] 4.3 Binary size cost of embedding the packaging icon, measured by
      building `knot` with and without it: 131,948,168 vs. 131,568,392 bytes
      (debug), a 379,776-byte difference matching the 368 KB asset - the
      fraction of a megabyte the design assumed.
