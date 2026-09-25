# Proposal

## Why

Users have no in-app way to report a problem. The About window already shows
version and build information precisely so it can be pasted into a bug report,
but there is no path from "I hit a bug" to "a GitHub issue exists" - the Help
menu carries a single disabled item. Knot runs on a machine with `gh` and the
user's GitHub credentials already available (the same assumption `knot-forge`
builds on), so the missing piece is a dialog that collects the report and a
submission path that uses those credentials.

## What Changes

- Add a **Report a Bug...** item to the Help menu, in the menu bar, beside the
  existing Knot Help item.
- Add a bug-report window: a dialog with a subject field, a description
  text area, and an app-diagnostics section that is filled in automatically.
- The diagnostics section shows the running app's version and build identifier
  (the same values the About window derives), the host OS and architecture,
  and whether the forge is reachable. Its contents are selectable so a user can
  copy them when they need to report out of band.
- Submitting the report opens a GitHub issue on the app's repository using the
  user's own credentials through the `gh` CLI, when `gh` is installed and
  authenticated (the capability `knot-forge` already probes for).
- When `gh` is missing or unauthenticated, submitting degrades to opening a
  pre-filled GitHub issue-compose page in the browser with the same subject,
  description and diagnostics - nothing is silently lost or skipped.
- Every piece of user-facing text comes from the localization catalog.

Out of scope: telemetry, crash reporting, automatic diagnostic collection
beyond the app-level identity and environment facts listed above, issue
templates beyond the single repository, and any interaction with GitHub beyond
creating the issue (no issue editing, listing, or deduplication).

## Capabilities

### New Capabilities

- `bug-reporting`: the in-app path from a Help menu item to a filed GitHub
  issue - the Report a Bug window, its subject/description fields, the
  diagnostics it attaches, and how a submission is delivered (via `gh` when
  available, browser fallback otherwise).

### Modified Capabilities

- `app-menu`: the Help menu gains a Report a Bug item that opens the bug-report
  window. Spec-level behavior of the menu bar changes; no key equivalent is
  added (GitHub has none to give it).

## Impact

- `crates/knot` (binary): a new `bug_report` window module modeled on
  `about_window`; the Help menu gains an item in `app_bootstrap.rs`; a handler
  registers on the window that owns the behavior, the same way
  `register_about_action` does.
- `crates/knot-forge`: gains an issue-creation operation that shells out to
  `gh issue create`, reusing `ForgeRunner`/`GhRunner`, the timeout discipline,
  and `ForgeAvailability` probing; or a thin sibling that composes the fallback
  issue URL.
- `crates/knot-core`: new localization keys for the dialog and its labels;
  a constant for the app's GitHub repository (which
  `https://github.com/pilgrimagesoftware/Knot` already names in `Cargo.toml`).
- `crates/knot/src/about_window/build_info.rs`: `version` and
  `build_identifier` are reused, not duplicated.