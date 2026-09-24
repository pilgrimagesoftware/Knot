# Design

## Context

See proposal.md - Why. Constraints that shape the approach:

- The app already runs on a machine with `gh` holding the user's GitHub
  credentials, and already treats it as the forge client: `knot-forge` reads
  pull request state through `GhRunner` behind a `ForgeRunner` trait, probes
  availability with `ForgeAvailability::probe()`, and deliberately never
  stores credentials (crates/knot-forge/src/lib.rs).
- The menu bar is declared in one place, `crates/knot/src/app_bootstrap.rs`;
  the Help menu currently carries a single disabled "Knot Help" item.
- `about_window/build_info.rs` already computes the running binary's version
  and build identifier - the two values a bug report must name - and its
  window shows how the report-like surface is built and registered.
- gpui-kit 0.6.4's `Dialog::content(impl IntoElement)` lets a dialog embed
  arbitrary content, so a form with text fields inside a modal dialog is
  available without a custom window; the app already opens modal dialogs via
  `Root::open_dialog` / `win.open_alert_dialog` (quit_guard).
- The app opens URLs in the browser through one helper, `crate::open_in::open_url(url) -> bool`.
- The app's GitHub repository is already named in `Cargo.toml`:
  `https://github.com/pilgrimagesoftware/Knot`.

## Goals / Non-Goals

**Goals:**
- File a GitHub issue from inside the app with the user's existing `gh`
  credentials, with a browser fallback that loses nothing.
- Reuse `knot-forge`'s runner, timeout, error and availability machinery
  rather than adding a credential store or an HTTP client.
- Reuse `about_window::build_info` for version/build instead of duplicating it.

**Non-Goals:**
- Telemetry, crash reporting, or any diagnostic beyond identity and environment
  facts collected in the dialog.
- Any GitHub interaction beyond creating an issue: no issue lists, edits, or
  dedup. No GitHub Enterprise targets for the report (the issue always goes to
  the app's own repo; `gh` choosing a different host via `-R owner/repo` is
  what routes it).
- Persisting drafts of a report across launches.

## Decisions

### 1. A modal dialog owned by the `knot` binary, not a window

The report is a structured, single-purpose task that ends - `Report` up front
with the diagnostics attached, `Cancel` to walk away. That is the modal
dialog's shape, and the spec mandates it: "opened as a dialog attached to the
active window."

The About window is a non-modal single-instance window, and deliberately so:
an About box blocks nothing because nothing collects anything. The bug report
dialog is the opposite, so it reuses neither the window entity nor the
single-instance handle that About keeps.

**Alternatives considered:** a non-modal window modeled on the About window
(rejected: a report half-typed underneath another window invites abandonment;
the reference pattern for "collect then submit" is the dialog); a custom
`open_window` with the form (rejected: `Dialog::content` exists exactly for
embedded content and brings modal + Escape + overlay for free).

The dialog is opened through `window.open_dialog(...)` on the active window,
falling back to the first open window, deferring like `quit_guard::request_quit`
does - the menu dispatch runs inside the active window's update, and a
re-entrant `window.update` is reported as a missing window. No window at all
logs and no-ops, the same lesser-harm justification quit_guard uses.

### 2. New `bug_report` module in `crates/knot`, shaped like `about_window`

```text
crates/knot/src/bug_report/
├── mod.rs         // register_report_bug_action, ReportBugState (form + result)
├── form.rs        // subject/description state + enablement rules (pure, testable)
├── diagnostics.rs // the diagnostics block: version/build/OS/arch/forge state
└── submit.rs      // availability dispatch: gh create, or browser fallback URL
```

`register_report_bug_action` follows `register_about_action`: it installs the
`ReportBug` action handler, resolves the target window, and opens the dialog
deferred. The dialog's `content` renders the subject field, description area,
diagnostics pane, and a footer of Cancel / Report buttons constructed from
`DialogButtonProps`-style props; the Report button's enabled state comes from
`form::ReportForm` (subject and description both non-empty, not whitespace).

**Why not extend `about_window`:** About is a read-only single window; the bug
report is a collect-and-submit dialog. They share only the version/build
string, which is already shared through `build_info`.

### 3. Issue creation lives in `knot-forge` as a new `issue` module

`knot-forge` gains `issue::create(runner, repo, title, body) -> Result<String>`
(and a `.with_runner`-style default over `GhRunner`), which runs
`gh issue create --repo <repo> --title <title> --body <body>` through the
existing `ForgeRunner`/`GhRunner` machinery. Title and body are passed as
separate argv elements through `Command` - never assembled into a shell line,
so no quoting problem and no injection path. The `<repo>` is `owner/repo`, and
`gh` resolves which host from the credentials it holds, exactly as pull
request state reads already do.

Availability is probed with the existing `knot_forge::probe()`; the three
states (Missing / Unauthenticated / Ready) map to the dialog's forge-readiness
diagnostic and to the submit branch.

**Alternatives considered:** an HTTP client with a stored token (rejected: the
same rationale lib.rs documents for pull requests - `gh` already holds the
credentials, knows Enterprise hosts, refreshes its own tokens; storing a token
is a security shelf this app does not need); a new crate (rejected: this is
forge behavior, `knot-forge` is its home).

### 4. Availability dispatch on Report

The dialog never guesses: it probes at open (for the diagnostics line) and
again at submit.

- **Ready** -> `submit::submit_with_gh(...)`: run `issue::create` off the main
  thread (tokio `spawn_blocking`; the runner is deliberately runtime-agnostic
  like `knot-git`), then on success show the confirmation and close the dialog,
  on `ForgeError` keep the dialog open, show the error, and leave subject,
  description and diagnostics intact so the user can retry or copy.
- **Missing / Unauthenticated** -> `submit::submit_via_browser(...)`: build the
  pre-filled issue
  URL `https://github.com/pilgrimagesoftware/Knot/issues/new?title=…&body=…`
  with both fields and the diagnostics block URL-encoded, and hand it to
  `crate::open_in::open_url`. The dialog shows the "ready to post in the
  browser" note and keeps everything in place. If `open_url` returns false,
  report the failure and keep the fields.

The app's repository is a `knot-core` constant (`KNOT_REPO` derived from the
same value Cargo.toml declares), shared by the gh path and the URL path so the
two cannot drift.

### 5. Diagnostics block: build_info plus one OS-version read

`diagnostics.rs` composes a selectable block:

```
Knot 0.x.y (build, <date|unknown commit>)
macOS <productVersion> (<kernel>), <arch>
Forge: <ready|not installed|not authenticated>
```

Version and build come from `about_window::build_info` (nudged from
`pub(super)` to `pub(crate)` so both windows share the single source of
truth). Arch is `std::env::consts::ARCH`; OS name is `std::env::consts::OS`.
The macOS product version is read once at dialog open with a `sw_vers
-productVersion` subprocess (matching the subprocess pattern the app already
relies on), falling back to the kernel release from `utsname` when the call
fails; forge state comes from `knot_forge::probe()`.

**Alternative considered:** read the kernel release only (rejected: "Darwin
24.6.0" without the product version reads as a version Knot itself controls;
the OS line is part of what a maintainer needs to triage).

### 6. Localization first

All dialog, menu, diagnostics-label and message strings are new keys under
`bug_report.*` in `crates/knot-core/locales/en.yml` (plus `menu.help.report_bug`
for the menu item), resolved through `knot_core::l10n::t` / `t_with`, matching
the `about.*` precedent and the l10n catalog test in
`crates/knot/src/tests/l10n_catalog.rs`. Prose values (version, build, OS,
arch) are not translated, but the labels around them are.

### 7. The Help menu item

`app_bootstrap.rs` gains a `ReportBug` action and a
`MenuItem::action(t("menu.help.report_bug"), ReportBug)` beside Knot Help. No
key equivalent, enabled always - matching the app-menu delta. The
`actions!`/UNWIRED machinery is untouched; `ReportBug` is wired immediately,
like `AboutKnot`.

## Risks / Trade-offs

- [Dialog modal may block a busy window] -> Reporting is short and modal is
  what the spec asks for; Escape exists from the first keypress, and Report/Cancel
  resolve it. The About window stays the place for leisurely reading.
- [`gh issue create` can fail for reasons availability miss (network, permissions)] ->
  `ForgeError::Command` maps to an in-dialog error with the form preserved; the
  user can retry or copy-and-paste. This is the pull-request-tracking crate's
  stated degradation philosophy applied to submission.
- [Compiler-injected args for title/body could exceed platform arg limits for huge bodies]
  -> a bug-report body is bounded in practice; if it ever grows unbounded this
  would switch to `--body-file -` with stdin, which `GhRunner` does not
  currently feed. Noted, not built.
- [Hardcoding the app repository couples the report to one target] -> the
  constant is `knot-core`'s single declaration and matches `Cargo.toml`; the
  design has no UI for "which repo" because issue filing on the app's own
  repository is the only behavior the spec needs.
- [URL-encoding mishaps in the browser fallback] -> the compose URL is built
  by one pure function with the same encoding used everywhere else, unit-tested
  with the literal Unicode/space/slash cases, and validated by the clickable
  scenario.

## Migration Plan

No migration: a new capability behind a new menu item, additive to the Rust
port. Rollback is removing the menu item and module, or simply not applying
the change. Deployment is the normal `cargo build` / `cargo test` gate; the
`knot-forge` and `knot-core` changes are additive (new module, new keys, a new
const), so they do not disturb the pull-request path.

## Open Questions

None that would change the specs, the approach, or the task breakdown. The two
facts a design could have deferred - the target repository
(`pilgrimagesoftware/Knot`, from Cargo.toml) and the OS-version source
(`sw_vers`) - were resolved above rather than pushed into tasks.