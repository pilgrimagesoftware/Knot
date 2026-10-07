# Proposal

## Why

Issue #566: a bug report or feature request is often clearer with a
screenshot, and the Report an Issue dialog has no way to include one.

GitHub has no API for uploading a file to an issue: `gh issue create` and the
REST API take text only, and image uploads go through the web UI. Logs get
around this by going into the body as text. A screenshot can't, because it is
binary.

## What Changes

- The dialog gets a **Screenshots** section, for bugs and feature requests
  alike. **Add Screenshots…** opens the native file picker with multiple
  selection. Image files (PNG, JPEG, GIF) are kept, up to a cap. Anything
  else is skipped, and the dialog says so. Each chosen file is listed by name
  with a remove control.
- The issue body names each screenshot under a **Screenshots** heading, as
  something the reporter will add.
- Delivery puts the user one drag away from attaching them:
  - **Filed through `gh`:** Knot opens the new issue's page in the browser
    and reveals the screenshots in Finder, so they can be dragged into a
    comment.
  - **Browser fallback:** Knot reveals the screenshots in Finder beside the
    pre-filled compose page.
  - The confirmation or status line says what to do.
- No screenshot is chosen by default, and none is sent anywhere except by
  the user's own drag.

## Capabilities

### Modified Capabilities

- `bug-reporting`: a new requirement, "A report can carry screenshots".

## Impact

- `crates/knot/src/bug_report/`: a new `screenshots.rs` (choosing, filtering
  and naming), with the dialog's section, `Report.screenshots`, delivery in
  `submit.rs`, and a screenshots-aware browser-ready status in `form.rs`.
- `crates/knot/src/consts.rs`: the screenshot cap and the accepted
  extensions.
- `crates/knot-core/locales/en.yml`: the section's labels, the body heading,
  and the status and notification text.
