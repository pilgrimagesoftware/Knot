# Tasks

## 1. The styling

- [ ] 1.1 In `render_tool_call_card`, apply
      `overflow_hidden().whitespace_nowrap().text_ellipsis()` to the title
      element when `collapsed` is true, leaving the expanded styling
      untouched. Verify in the app that a collapsed call with a long title
      renders on one line with an ellipsis and the status indicator still
      visible.
- [ ] 1.2 Confirm an expanded long-title call still wraps the header as it
      does today; verify in the app that expanding a collapsed card returns
      it to the previous layout.

## 2. Verification

- [ ] 2.1 Confirm the header's hit target and disclosure behavior are
      unchanged; verify the whole header still toggles the card.
- [ ] 2.2 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 2.3 Exercise in the app with a long command title at a narrow window:
      collapsed shows one line with an ellipsis, the status indicator stays
      in place, and no conversation-layout shift occurs on collapse.