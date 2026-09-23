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

### Not yet walked

- **3.4, the rest** — IME composition, selection by mouse, and undo/redo.
  Undo/redo is the one to weight: `gpui-base`'s `enter()` calls
  `undo_manager.break_transaction_coalescing()` on the submit path, and the
  swap changed which branch of that function the composer takes.
- **5.1/5.4 in the other appearance** — the light/dark switch restyling with
  no edit.
- **8.2** — the full scenario sweep, once groups 6 and 7 land.
