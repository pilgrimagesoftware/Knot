# Tasks

## 1. Settle how the family reaches the label

- [x] 1.1 Apply `.font_family(...)` to the `Button` returned by
      `font_picker_button` in `crates/knot/src/settings_window/mod.rs` with a
      hard-coded family, and verify in the app that the button's label is
      drawn in it. If `Styled` on `Button` does not reach the label, switch to
      passing a styled `div` as the button's child instead of `.label()`, per
      design.md, and note which route was taken in a comment.

## 2. The preview

- [x] 2.1 Give `font_picker_button` the `&App` it needs and draw each row's
      label in the family it names. Verify in the app that the UI, Title and
      Terminal rows each render in their own face, and that two rows set to
      different families look different.
- [ ] 2.2 Verify the label keeps the settings window's text size: set the
      Title font to 48pt and confirm its row reads "48pt", is drawn at the
      same size as the other two rows, and leaves all three row heights
      equal.
- [ ] 2.3 Verify the preview follows a new choice: pick a family in the font
      panel and confirm the row redraws in it without reopening the settings
      window.

## 3. Unresolvable fonts

- [x] 3.1 Test the family against `cx.text_system().all_font_names()` before
      applying it, matching `terminal_font_family`, and fall back to the
      default face when it does not resolve. Verify by setting a font name
      that is not installed and confirming the row is not drawn in a
      substitute face.
- [x] 3.2 Mark an unresolvable row in its label while still naming the
      persisted family. Verify in the app that the row reads the configured
      family plus the marking, and that a resolvable row carries no marking.
- [ ] 3.3 Verify the persisted value is untouched: with a row marked
      unavailable, confirm the settings file still holds the configured
      family and that the marking clears on its own once the family resolves.

## 4. Tests

- [x] 4.1 Add a unit test for the pure part - given a family, a size and the
      set of resolvable names, what family the label should use and whether
      it is marked - covering a resolvable family, an unresolvable one, and
      an empty family name. Extract that decision into a free function if it
      is not already one, so it is testable without a window.

## 5. Verification

- [x] 5.1 Run `make rust` and verify fmt, clippy, tests and build all pass
      for the workspace.
- [ ] 5.2 Open the Appearance tab with all three fonts set to different
      installed families and confirm the section reads as three distinct
      faces at one size, then set one to a font that is not installed and
      confirm only that row is marked.

## Outstanding in-app verification

The checked tasks above are implemented and covered by `make rust`; the
confirmations that need the running app - 2.2, 2.3, 3.3 and 5.2, and the
visual half of 1.1, 2.1, 3.1 and 3.2 - are still open. The route for 1.1 was
settled by reading `gpui-component`'s `Button::render` and `gpui`'s `Div`
paint instead: the label is a descendant div of the root the caller's style
refines, and a div applies its text style to its descendants, so `.font_family`
on the `Button` reaches the label and `.label()` did not have to be replaced
with a styled child.
