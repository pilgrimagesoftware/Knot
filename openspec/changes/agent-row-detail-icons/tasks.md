## 1. The icons

- [ ] 1.1 Give the status line a leading `Activity` icon in the row's muted
      colour, in the same `h_flex` shape the agent-type line uses. Verify in
      the app that it aligns with the icons above it.
- [ ] 1.2 Give the folder line a leading `Folder` icon the same way. Verify
      in the app.
- [ ] 1.3 Replace the persona line's `👤` text prefix with a drawn `User`
      icon, so all four detail lines use the same mechanism. Verify in the
      app that the persona icon now matches the others in colour and size.

## 2. Sizing

- [ ] 2.1 Size each icon to its own line rather than fixing one size for the
      row - the type and persona lines are `text_xs`, the status and folder
      lines are the UI font size. Verify in the app that no icon reads as
      undersized or oversized beside its text.

## 3. Verification

- [ ] 3.1 `make rust` passes clean.
- [ ] 3.2 In the app, check a row with every line present, a row with no
      persona, and a shell companion - confirming no icon is drawn beside a
      line that is not there.
