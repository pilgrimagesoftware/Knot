## 1. Sequencing

- [ ] 1.1 Archive `acp-agent-panel-ui` so `acp-panel-ui` exists under
      `openspec/specs/`. Verify `openspec validate tool-call-card-styling
      --strict` reports no archive problem for this delta.

## 2. Theme colours reach the card

- [ ] 2.1 Add the three outline colours to `PanelStyle` - danger, info and
      the neutral card border - read from the theme at the call site that
      already reads `mono_font_family`. Verify the card no longer references
      `ERROR_COLOR` or `CARD_BORDER` for its outline.
- [ ] 2.2 Add a pure function from a card's status to which of the three
      applies. Verify with tests: `failed` takes danger, `pending` and
      `in_progress` take info, `completed` takes the neutral border, and an
      unrecognised status takes the neutral border rather than panicking -
      `status` is a wire string and a future value must not break the card.

## 3. The header

- [ ] 3.1 Render the title in `PanelStyle`'s monospace family and the status
      in the proportional one. Verify in the app that a shell command reads
      as a command and "Done" beside it does not.
- [ ] 3.2 Apply the outline colour from 2.2. Verify in the app across a turn
      that includes a running call, a completed one and a failed one.

## 4. Verification

- [ ] 4.1 `make rust` passes clean.
- [ ] 4.2 Switch the app between light and dark and confirm all three outline
      states follow the theme rather than staying fixed.
