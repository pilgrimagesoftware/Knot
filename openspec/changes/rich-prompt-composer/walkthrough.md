# Debug-build walkthrough

What was checked by hand in a debug build, and by whom. Tasks 3.4 and 8.2
ask for this record; the automated evidence lives in the test modules named
beside each entry.

The agent driving this change could not perform any of it: `screencapture`
fails with `could not create image from display` from the agent shell (no
Screen Recording permission), synthetic clicks are ignored, and GPUI renders
into a single `AXGroup` exposing no text. Menu structure and window state are
reachable via `System Events`; nothing visual is. See `~/Code/papercuts.md`,
entry of 2026-09-21.

## 2026-09-23 — styling, first look (maintainer)

Debug build of `396-rich-prompt-composer` at `40fc9d7`, launched with
`./target/debug/knot`.

| Task | Scenario | Result |
| --- | --- | --- |
| 5.1 | Each treatment visually distinct — heading, strong, emphasis, inline code, link, list marker, block quote, slash token, mention | **Pass.** No issue stood out. |
| 5.4 | Legible in the active appearance | **Pass.** |
| 3.4 | Context menu | **Pass.** |
| 3.4 | Selection by keyboard | **Pass.** |

Automated counterparts: `composer_style::tests` (no treatment is hue-only, no
two are drawn alike except the deliberate code pair, treatments change with
the palette) and `tests::panel_composer` (a restored draft is styled on
arrival, the buffer survives styling byte for byte).

A `/review` typed into the composer completed and dispatched normally, so the
slash lookup and the send path still work over `EditorState`.

## 2026-09-23 — the full sweep (maintainer)

Debug build at `e966fc8e`, with groups 6 and 7 in. Every scenario below was
walked and passed; nothing was raised.

| Spec | Scenario | Result |
| --- | --- | --- |
| panel-rich-input | Token styled from the trigger, before it resolves (`/rev`) | **Pass** |
| panel-rich-input | A slash inside a path is prose (`crates/knot/src`) | **Pass** |
| panel-rich-input | An email address is prose (`paul@example.com`) | **Pass** |
| panel-rich-input | Emphasis, strong, inline code, heading, list marker, block quote, link each distinct | **Pass** |
| panel-rich-input | An unclosed marker does not bleed (`a * b`) | **Pass** |
| panel-rich-input | A fenced block is one code treatment whatever its info string | **Pass** |
| panel-rich-input | A bracket is not auto-closed | **Pass** |
| panel-rich-input | A newline does not inherit the previous line's indent | **Pass** |
| panel-rich-input | No gutter, line numbers, indent guides or fold controls | **Pass** |
| panel-rich-input | Styling survives paste, undo, redo and cut | **Pass** |
| panel-rich-input | An appearance switch restyles with no edit | **Pass** |
| panel-file-mentions | `@` after a space opens the lookup; mid-word does not | **Pass** |
| panel-file-mentions | The gathering state shows, and typing continues through it | **Pass** |
| panel-file-mentions | A subsequence matches (`@kgs` reaches `knot-git/src/lib.rs`) | **Pass** |
| panel-file-mentions | Name matches outrank directory matches | **Pass** |
| panel-file-mentions | Matched characters are marked in the row | **Pass** |
| panel-slash-commands | The caret moving between a `/` and an `@` token switches the list, never both | **Pass** |
| acp-panel-ui | All three arrival paths produce a chip and a strip row | **Pass** |
| acp-panel-ui | Deleting the chip removes the strip entry | **Pass** |
| acp-panel-ui | Removing the strip entry removes the chip | **Pass** |
| 3.4 | Undo/redo after the `Editor` swap | **Pass** |

### Still not walked

- **3.4, the remainder** — IME composition and selection by mouse. Neither is
  reachable from a test: GPUI exposes no text through the accessibility tree
  and the agent shell cannot post synthetic pointer events.
