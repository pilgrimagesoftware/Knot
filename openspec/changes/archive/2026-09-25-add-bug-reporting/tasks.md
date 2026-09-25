# Tasks

## 1. Forge issue creation (`knot-forge`)

- [x] 1.1 Add `issue` module (and `REPO` arg shape) to `knot-forge` exposing
      `issue::create(runner, repo, title, body) -> Result<String>`, running
      `gh issue create --repo <repo> --title <title> --body <body>` through
      the existing `ForgeRunner`/`GhRunner` machinery with title/body passed as
      separate `Command` argv elements, returning the filed issue's URL on
      success; verify `cargo test -p knot-forge` passes with a stub runner
      asserting the exact argv and that success yields the URL
- [x] 1.2 Add `ForgeError` coverage tests for the issue path (timeout and
      `Command` failure map through unchanged); verify
      `cargo test -p knot-forge` passes
- [x] 1.3 Export the new module and its types from
      `knot-forge/src/lib.rs` and run `cargo clippy -p knot-forge` clean
      (`-D warnings`)

## 2. Shared constants and localization (`knot-core`)

- [x] 2.1 Add the app's GitHub repository constant (`KNOT_REPO`, value
      `pilgrimagesoftware/Knot`, matching the Cargo.toml `repository` URL's
      owner/repo) to `crates/knot-core/src/consts.rs`; verify a unit test
      asserts it parses as `owner/repo` with no scheme and no trailing slash
- [x] 2.2 Add `bug_report.*` keys to `crates/knot-core/locales/en.yml` for the
      menu item (`menu.help.report_bug`), window title, subject and
      description labels and placeholders, diagnostics labels, forge-state
      strings (not installed / not authenticated / ready), the success and
      failure messages, the browser-fallback note, and the Cancel / Report
      controls; verify every key resolves via `knot_core::l10n::t` in a test
      and the file stays valid YAML (`cargo test -p knot-core -p knot`)

## 3. Diagnostics block (`crates/knot`)

- [x] 3.1 Create `bug_report/diagnostics.rs` composing the selectable
      diagnostics block: app name + version + build identifier (reusing
      `about_window::build_info`, promoted to `pub(crate)`), OS name and
      architecture from `std::env::consts`, the macOS product version read
      once via `sw_vers -productVersion` with kernel-release fallback, and
      forge state from `knot_forge::probe()`; verify unit tests cover the
      block layout, the fallback path, and that the version/build string is
      byte-identical to `build_info::build_details()`
- [x] 3.2 Add pure formatting helpers (localized labels via
      `knot_core::l10n::t`, no inline prose) and verify the l10n catalog test
      in `crates/knot/src/tests/l10n_catalog.rs` passes with the new keys

## 4. Report form and dialog (`crates/knot`)

- [x] 4.1 Create `bug_report/form.rs` with subject/description state and pure
      enablement rules (Report enabled only when both are non-empty and not
      whitespace-only, disabled mid-submission); verify unit tests cover
      empty, whitespace-only, valid, and in-flight cases
- [x] 4.2 Create `bug_report/mod.rs` and `register_report_bug_action` wiring
      the `ReportBug` action to open a modal dialog via the active window
      (`open_dialog`) with the form content, diagnostics pane, Cancel and
      Report controls, and Escape-to-cancel, falling back to the first open
      window and deferring the open like `quit_guard::request_quit`; verify
      tests assert the dialog layer renders (debug selector), Escape closes
      it, and opening again while open is handled without a second dialog
- [x] 4.3 Handle the "no window to attach to" case by logging and no-oping
      rather than crashing; verify a test with no windows runs without panic

## 5. Submission dispatch (`crates/knot`)

- [x] 5.1 Create `bug_report/submit.rs` with `submit_with_gh`: probe
      `knot_forge::probe()`, run `issue::create` on a background task
      (`spawn_blocking`), on success show the confirmation and close the
      dialog, on `ForgeError` keep the dialog open with the error shown and
      the subject/description/diagnostics intact and Report re-enabled;
      verify unit tests cover success, failure-preserves-form, and
      retry-after-failure
- [x] 5.2 Add `submit_via_browser`: build the pre-filled compose URL
      `https://github.com/{KNOT_REPO}/issues/new` with title, description and
      diagnostics URL-encoded, hand it to `crate::open_in::open_url`, show the
      "ready to post in the browser" note, and report a failure (keeping the
      fields) when `open_url` returns false; verify unit tests cover the URL
      encoding (spaces, slashes, Unicode) and the open-failure branch
- [x] 5.3 Ensure the URL builder is one pure function with both dispatch
      branches using the same `KNOT_REPO` constant; verify tests assert the
      gh path and URL path cannot drift on the repo value

## 6. Menu integration (`crates/knot`)

- [x] 6.1 Add a `ReportBug` action and a Help-menu
      `MenuItem::action(t("menu.help.report_bug"), ReportBug)` beside Knot
      Help in `app_bootstrap.rs`, enabled and with no key equivalent, and
      register it via `register_report_bug_action`; verify
      `cargo test -p knot` and the menu key-equivalents test
      (`tests/menu_key_equivalents.rs`) pass with the new item asserting no
      key equivalent is assigned

## 7. Verification

- [x] 7.1 Run the full gate and confirm it passes:
      `make fmt-check`, `make size-check`, `make lint` (clippy `-D warnings`),
      `make test`, `make build`
- [x] 7.2 Read the `bug-reporting` and `app-menu` specs against the
      implementation and confirm each requirement and scenario is satisfied,
      then run `openspec validate` for the change