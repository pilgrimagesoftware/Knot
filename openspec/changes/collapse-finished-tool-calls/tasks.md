## 1. Sequencing

- [x] 1.1 Archive `acp-agent-panel-ui`, whose implementation is merged, so
      `acp-panel-ui` exists under `openspec/specs/`. Verify
      `openspec validate collapse-finished-tool-calls --strict` reports no
      archive problem for this delta.

## 2. The rule

- [x] 2.1 Add the per-call override map to `PanelState`, a `toggle` that
      records an explicit choice, and a pure `is_collapsed(card)` reading
      override-then-default. Verify with tests over every combination:
      untouched and running (expanded), untouched and completed (collapsed),
      untouched and failed (expanded), opened then completed (expanded),
      closed then completed (collapsed), closed then failed (collapsed).
- [x] 2.2 Confirm the overrides reset with the rest of the panel's view
      state when a conversation is reloaded. Verify with a test that a fresh
      `PanelState` collapses a completed call the previous one had open.

## 3. The card

- [ ] 3.1 Render the header's disclosure control, reflecting
      `is_collapsed`, and hide the content when collapsed. Verify in the app
      that a succeeding call folds to its header and a failing one does not.
- [ ] 3.2 Make the header row toggle the card, not only the chevron. Verify
      in the app by clicking the title.
- [ ] 3.3 Confirm collapsing does not move the viewport: with auto-scroll
      following, the panel stays at the end; with the user scrolled back, the
      content they are reading stays put. Verify in the app with a turn long
      enough to scroll.

## 4. Verification

- [x] 4.1 `make rust` passes clean.
- [ ] 4.2 In the app, run a turn with several tool calls and confirm the
      conversation reads as the agent's replies with folded calls between
      them, and that opening one leaves the rest alone.
