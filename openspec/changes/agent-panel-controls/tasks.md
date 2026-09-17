## 1. Tool-call icons

- [x] 1.1 Add an icon lookup keyed on `ToolCallCard.kind` (exhaustive
      match, generic fallback arm) in `panel_view.rs`. Verify: unit test
      covers every `ToolCallKind` variant plus the fallback arm.
- [x] 1.2 Render the icon in `render_tool_call_card`, preceding the
      existing kind/summary label. Verify: manual run shows an icon on
      every tool-call card in a live ACP session.

## 2. Response action bar

- [ ] 2.1 Add a response action bar component (copy, scroll-to-user,
      scroll-to-top) rendered under `PanelMessage::Assistant` once
      streaming has ended. Verify: unit test asserts the bar is absent
      while a message is still streaming and present once finalized.
- [ ] 2.2 Wire "copy response" to the system clipboard with the message's
      full text. Verify: manual run confirms clipboard contents match the
      rendered response.
- [ ] 2.3 Wire "scroll to user input" and "scroll to top" to the panel's
      scroll container. Verify: manual run in a multi-turn conversation.

## 3. Track toggle

- [ ] 3.1 Add per-response track state and an auto-scroll subscription
      that follows new content for the tracked response while streaming.
      Verify: unit test simulates streamed deltas and asserts scroll
      offset tracks the bottom.
- [ ] 3.2 Detect user-initiated scroll away from bottom and clear tracking.
      Verify: unit test simulates a scroll event mid-stream and asserts
      the track toggle turns off.

## 4. Input-area control bar

- [ ] 4.1 Add the input-area component (text entry + control row) as a
      new sibling under `render_panel`. Verify: `cargo build -p knot`
      succeeds and the bar renders in a live session.
- [ ] 4.2 Add the add-context control (file/image picker via
      `cx.prompt_for_paths`, per `knot-ui-conventions`' native-picker
      rule) and attach the result to the pending message. Verify: manual
      run attaches a file and it appears in the input area before send.
- [ ] 4.3 Add the permission-mode selector, sourced from the session's
      available modes; disable with a tooltip when the adapter reports
      none. Verify: unit test covers the populated and disabled cases.
- [ ] 4.4 Add the model selector, sourced from session capabilities, same
      disabled-with-tooltip fallback. Verify: unit test covers populated
      and disabled cases.
- [ ] 4.5 Add the effort selector, same pattern as 4.3/4.4. Verify: unit
      test covers populated and disabled cases.
- [ ] 4.6 Add the send control: disabled while input is empty, and while a
      turn is in flight if the session doesn't support concurrent input.
      Wire it to submit text plus attached context via `session/prompt`
      and clear the input area. Verify: unit test covers both disabled
      conditions and a manual run confirms send-and-clear.

## 5. Input area expand/collapse

- [ ] 5.1 Add the expand toggle and a larger multi-line editing size for
      the input area, collapsing back on second activation. Verify: manual
      run toggles between both sizes without losing in-progress text.

## 6. Verification pass

- [ ] 6.1 Run `make rust` (fmt + clippy + test + build) and fix any
      failures introduced by this change. Verify: command exits 0.
- [ ] 6.2 Manual pass against specs/acp-panel-ui/spec.md's new scenarios
      (tool icon, response action bar, track toggle, input-area controls,
      expand/collapse) in a live ACP session.
