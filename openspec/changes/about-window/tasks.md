# Tasks

## 1. Build stamping

- [ ] 1.1 Add `crates/knot/build.rs` emitting `KNOT_BUILD_COMMIT` (short git
      SHA, `-dirty` suffix when the tree is dirty, `unknown` when `git` is
      unavailable or fails) and `KNOT_BUILD_DATE` (UTC `YYYY-MM-DD`), with
      `rerun-if-changed` on `.git/HEAD` and the ref it points at; add `time`
      as a `[build-dependencies]` entry from the workspace. Verify with
      `cargo build -p knot` succeeding and the values reaching the binary
      (task 2.2's test covers the read).
- [ ] 1.2 Verify the tarball path: with `PATH` stripped of `git`, or from a
      copy of the tree with no `.git`, `cargo build -p knot` still succeeds
      and stamps `unknown`.

## 2. About window content

- [ ] 2.1 Add the `about:` strings to `crates/knot-core/locales/en.yml` -
      window title, version label, build label, copyright, and one credits
      key per attributed item (author, app license, bundled fonts, UI
      toolkit, terminal engine). Verify each key resolves through
      `knot_core::l10n::t` in a unit test rather than falling back to the key
      string.
- [ ] 2.2 Add `crates/knot/src/about_window/mod.rs` with the build-identity
      values (version from `CARGO_PKG_VERSION`, build from the two stamped
      env values) exposed as a small function returning the display strings,
      and a unit test asserting the version matches the crate version and the
      build string is non-empty in both the stamped and `unknown` cases.
- [ ] 2.3 Implement the `AboutWindow` view: icon (embedded
      `assets/icon/icon.png`, rendered at a fixed size), app name, version
      and build line, copyright line, credits block, in that order. Verify by
      building and by the window test in 3.3.
- [ ] 2.4 Add the copy affordance on the version/build line writing
      `"Knot <version> (<build>)"` to the clipboard. Verify with a unit test
      over the formatting function (the clipboard call itself is a one-liner
      over the same string).
- [ ] 2.5 Add the `#[cfg(not(target_os = "macos"))]` Close button calling
      `window.remove_window()`. Verify the crate builds for a non-macOS
      target (`cargo check -p knot --target x86_64-unknown-linux-gnu` if the
      target is installed; otherwise confirm the `cfg` arms compile under
      `cargo clippy --workspace --all-targets`).

## 3. Opening and dismissing the window

- [ ] 3.1 Add `about_window_options` to `window_options.rs`: fixed bounds,
      `is_resizable: false`, `is_minimizable: false`, titlebar titled from
      the catalog. Verify the window opens at that size and refuses to
      resize when dragged (manual, task 4.2).
- [ ] 3.2 Replace `about_knot`'s alert-dialog body with
      `open_about_window(handle, cx)`: raise the existing window when its
      update succeeds, otherwise open a new one and store the handle; register
      `AboutKnot` in `run()` as a closure over a new
      `Rc<RefCell<Option<AnyWindowHandle>>>`, mirroring `OpenSettings`.
      Verify with the tests in 3.3.
- [ ] 3.3 Rewrite `crates/knot/src/tests/about_dialog.rs` as
      `about_window.rs`: dispatching `AboutKnot` opens exactly one window;
      dispatching it a second time opens no second window; dispatching after
      the window is removed opens one again. Verify `cargo test -p knot`
      passes and update `tests/mod.rs`.
- [ ] 3.4 Handle `Escape`: track a `FocusHandle` on the view root, focus it
      when the window opens, and close the window from an `on_key_down`
      listener matching the `escape` keystroke. Verify manually (task 4.2) -
      the headless test platform does not deliver key events to a real
      window.
- [ ] 3.5 Update the `root_overlays` doc comment in `app_support.rs`, which
      names About as one of the dialogs that opens through the dialog layer -
      no longer true once About is its own window.

## 4. Verification

- [ ] 4.1 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 4.2 Manual macOS verification: (a) About Knot opens the window and the
      app stays usable behind it; (b) choosing About again raises the same
      window instead of opening a second; (c) closing it and choosing About
      reopens it; (d) `Escape` and the close button both dismiss it; (e) the
      window refuses to resize and has no minimize; (f) the version and build
      shown match `cargo pkgid -p knot` and `git rev-parse --short=12 HEAD`;
      (g) the copy affordance puts the version and build on the clipboard;
      (h) the credits name the author, the app's license, and each embedded
      third-party work.
- [ ] 4.3 Record the binary size change from embedding the packaging icon
      (`cargo build --release -p knot`, compare the binary size before and
      after) and confirm it is within the fraction of a megabyte the design
      assumes.
