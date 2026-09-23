# Tasks

## 1. The hover group

- [x] 1.1 In `crates/knot/src/panel_view/message.rs`, wrap the copy button and
      the prompt bubble in a shrink-wrapped `h_flex()` inside the existing
      full-width `justify_end()` row, and give that inner row `.group(..)` with a
      module-level `&'static str` constant; verify with `make build` and by
      confirming a prompt still renders right-aligned with the button beside it
- [x] 1.2 Make the button `.invisible()` with
      `.group_hover(GROUP, |style| style.visible())`, with a comment saying
      `Visibility::Hidden` keeps the button in the layout so revealing it cannot
      reflow the conversation; verify with `make lint` and by hovering a prompt
- [x] 1.3 Confirm the constant group name scopes per message rather than
      globally — this is the one API assumption in design.md: open a conversation
      with several prompts, hover one, and check that only that prompt's control
      appears

## 2. Walk the spec

- [x] 2.1 Hover a prompt bubble and confirm the control appears; move the pointer
      off and confirm it goes, with the bubble unchanged
- [x] 2.2 Move the pointer from the bubble onto the revealed control and confirm
      it stays shown and still copies — the case the old wording would have
      broken
- [x] 2.3 Rest the pointer in the empty area left of a short prompt, on the same
      line, and confirm nothing is revealed
- [x] 2.4 Move the pointer down a conversation across several prompts and confirm
      nothing shifts position as controls appear and disappear
- [x] 2.5 Confirm copying is unchanged: the tooltip, the clipboard text (prompt
      only, no attachment payload), and the confirmation notification

## 3. Check nothing else moved

- [x] 3.1 Confirm the code block copy control is still drawn without a hover,
      per `acp-panel-ui`'s "The control does not wait for a hover" — the two
      controls are deliberately different and this change must not have touched
      the other one
- [x] 3.2 Confirm the response action bar is unchanged
- [x] 3.3 Scroll a long conversation with the pointer held still over the list
      and confirm no control is left stuck shown on a row that scrolled past it
- [x] 3.4 Run `make` and confirm the whole gate passes — `fmt-check`,
      `size-check`, `clippy -D warnings`, tests, build
