# Tasks

## 1. The control

- [ ] 1.1 Add a hover-revealed ghost copy button to the `PanelMessage::User`
      arm of `render_message`, placed on the message row left of the bubble,
      copying the message text with
      `cx.write_to_clipboard(ClipboardItem::new_string(text.clone()))`.
      Verify in the app that hovering a prompt reveals it and clicking puts
      the prompt text on the clipboard.
- [ ] 1.2 Give the control a region-scoped element id (`index`-based, like
      the markdown id) so per-prompt hover and click state cannot collide
      across messages in one conversation.

## 2. Behavioural edges

- [ ] 2.1 Confirm the hover surface is the whole message row, not the bubble
      rect; verify in the app with a short and a multi-line wrapped prompt
      that the control arms reliably and hides on pointer leave.
- [ ] 2.2 Verify the copied text is exactly the prompt text - paste from the
      clipboard after copying a prompt that was sent with attachments and
      confirm no attachment payload is included.
- [ ] 2.3 Verify an assistant response's copy still behaves as before
      (regression check against `panel-copy-response`).

## 3. Polishing and verification

- [ ] 3.1 Route the control's tooltip through `knot_core::l10n::t`.
- [ ] 3.2 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 3.3 Exercise in the app: send a multi-line prompt, hover, copy, paste
      elsewhere, and confirm the pasted text matches the prompt exactly.