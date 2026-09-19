# Tasks

## 1. Classifier

- [ ] 1.1 Add an `is_image_path` helper (extension set modeled on the paste
      path) and unit tests covering the known image extensions, a plain text
      file, and a file with no extension.
- [ ] 1.2 Add a small grouping helper that turns a `&[PathBuf]` into a
      `(files, images)` count pair, with unit tests for empty, files-only,
      images-only, and mixed lists.

## 2. The indicator

- [ ] 2.1 Render the context indicator pill - paperclip icon, file/image
      summary, tooltip listing the attached names, and the zero state reading
      "no context" - in the left of the input area's bottom control row.
      Verify in the app that it holds its place when the chips come and go.
- [ ] 2.2 Route its strings (zero state, "N files · M images") through
      `knot_core::l10n::t` and verify they appear in a locale/format check.

## 3. Clear-all

- [ ] 3.1 Add a clear-all control on the indicator that empties
      `panel_pending_context` for that agent id in one action, disabled while
      the list is empty. Verify in the app that one click clears several
      chips at once and that the next Send carries no attachments.
- [ ] 3.2 Verify the Send path uses the (now empty) pending list, so a
      cleared attachment is not delivered; exercise attach→clear→send with 2+
      items.

## 4. Verification

- [ ] 4.1 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 4.2 Confirm in the app that the indicator and the chips row disagree
      nowhere: adding/removing chips reflects instantly in the indicator, and
      both vanish together on Send.