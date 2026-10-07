# Design

## Context

`bug_report` files an issue with `gh issue create`, or opens GitHub's
pre-filled compose page when `gh` isn't ready (`submit.rs`). Logs go in the
body as fenced text. The browser fallback can't carry that much in a URL, so
it names the log files instead and the user attaches them on the page
(`bug-reporting`, "A report can attach the application and MCP logs").

## Decisions

**Name the screenshots in the body, then put the user one drag from adding
them.** No GitHub API takes a binary attachment, so Knot can't upload the
images itself. The alternatives each fail on their own terms:

- **Commit the images to a branch and link them:** needs write access to
  the Knot repository, which reporters don't have.
- **A gist:** gists are text-only.
- **GitHub's web upload endpoint:** undocumented and cookie-authenticated,
  and the spec forbids asking for credentials.

What Knot can do is what it already does for logs on the browser path: name
the files, and make adding them a drag. After a filed report it opens the
issue page and reveals the files in Finder. On the browser path it reveals
them beside the compose page.

**Pick files, don't capture.** The issue asks to attach screenshots the user
already has, and macOS users take them with ⌘⇧4. A capture control would mean
picking a window, handling Screen Recording permission, and choosing where
the file goes. That's a separate feature.

**Filter after picking.** GPUI's `PathPromptOptions` has no file-type filter,
so the picker shows everything. The chosen files are filtered by extension
(PNG, JPEG, GIF, the formats GitHub renders inline). Anything else is skipped
with a status line naming how many were skipped, rather than failing silently.

**Cap the count** (`BUG_REPORT_MAX_SCREENSHOTS`). A report wants "a couple",
and an accidental ⌘A in a screenshots folder shouldn't reveal a hundred
Finder selections.

**Delivery runs in `submit`, off the main thread.** `submit` already owns
the blocking `gh` and `open` calls. Opening the issue page and revealing
files are more of the same, behind the same kind of seam (`reveal`, beside
`open_url`), so tests run neither.

**Not persisted, not read.** The dialog holds only paths. Knot never opens
the image files, so nothing reads them on the render path, and the report
carries names only.

## Risks

- A user may not notice that the screenshots still need dragging in. The
  confirmation says so explicitly, and the issue body names the files, so a
  reader can see that something was meant to be attached.
- Revealing files is macOS-only (`open_in::can_reveal_files`). Elsewhere the
  names still go in the body, and the status says to attach them by hand.
